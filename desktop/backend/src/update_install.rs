use crate::{Result, updater::{self,Manifest}, storage, process, app_services, powershell};
use serde::{Serialize,Deserialize};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{fs::{self,File,OpenOptions}, io::{Read,Write}, path::{Path,PathBuf}, process::{Command,Stdio}, time::{Duration,Instant,SystemTime}};
use std::os::windows::{ffi::OsStrExt,fs::{MetadataExt,OpenOptionsExt},process::CommandExt};
use windows_sys::Win32::{Foundation::{CloseHandle,ERROR_ALREADY_EXISTS,GetLastError,HANDLE,INVALID_HANDLE_VALUE},Storage::FileSystem::{CreateFileW,FILE_FLAG_BACKUP_SEMANTICS,FILE_READ_ATTRIBUTES,FILE_SHARE_READ,FILE_SHARE_WRITE,OPEN_EXISTING},System::Threading::CreateMutexW};

#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Job { pub nonce:String,pub parent_pids:Vec<u32>,pub install_dir:PathBuf,pub user_dir:PathBuf,pub startup:bool,pub legacy_exe:Option<PathBuf>,#[serde(default)]pub portable:bool,#[serde(default)]pub portable_exe:Option<PathBuf> }
struct MigrationGuard(HANDLE);
impl Drop for MigrationGuard {fn drop(&mut self){unsafe{CloseHandle(self.0);}}}
struct PathGuard(HANDLE);
impl Drop for PathGuard {fn drop(&mut self){unsafe{CloseHandle(self.0);}}}
struct RemoveDirectory(PathBuf);
impl Drop for RemoveDirectory {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn migration_guard()->Result<MigrationGuard>{let name="Local\\NogiremRustMigration\0".encode_utf16().collect::<Vec<_>>();let handle=unsafe{CreateMutexW(std::ptr::null(),0,name.as_ptr())};if handle.is_null(){return Err(std::io::Error::last_os_error().to_string());}if unsafe{GetLastError()}==ERROR_ALREADY_EXISTS{unsafe{CloseHandle(handle);}return Err("다른 Rust 전환 작업이 이미 진행 중입니다".into());}Ok(MigrationGuard(handle))}
fn err(e:impl std::fmt::Display)->String{e.to_string()}
fn quote(s:&str)->String{format!("'{}'",s.replace('\'',"''"))}
pub fn installation()->Result<PathBuf>{Ok(PathBuf::from(std::env::var_os("ProgramW6432").or_else(||std::env::var_os("ProgramFiles")).ok_or("Program Files 누락")?).join("Nogirem"))}
fn user_dir()->Result<PathBuf>{Ok(PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA 누락")?).join("마비노기 렘 부스터"))}
fn expected_user(_job:&Job)->Result<PathBuf>{user_dir()}
fn same(a:&Path,b:&Path)->bool{a.to_string_lossy().replace('/',"\\").eq_ignore_ascii_case(&b.to_string_lossy().replace('/',"\\"))}
fn contained(path:&Path,root:&Path)->Result<()> {
    let p=path.canonicalize().map_err(err)?;let r=root.canonicalize().map_err(err)?;
    if p==r || !p.starts_with(&r){return Err("업데이트 작업 경로가 허용 범위를 벗어났습니다".into());}Ok(())
}
fn files(root:&Path)->Result<Vec<PathBuf>> {
    fn visit(root:&Path,dir:&Path,out:&mut Vec<PathBuf>)->Result<()> {
        for e in fs::read_dir(dir).map_err(err)? {let p=e.map_err(err)?.path();let m=fs::symlink_metadata(&p).map_err(err)?;
            if m.file_attributes()&0x400!=0 {return Err("설치 경로에 연결된 외부 폴더가 있습니다".into());}
            if m.is_dir(){visit(root,&p,out)?;}else if m.is_file(){out.push(p.strip_prefix(root).map_err(err)?.into());}
        }Ok(())
    }
    let mut out=vec![];if root.exists(){visit(root,root,&mut out)?;}Ok(out)
}
fn copy_files(from:&Path,to:&Path,list:&[PathBuf])->Result<()> {
    for name in list {if name.is_absolute() || name.components().any(|c|matches!(c,std::path::Component::ParentDir|std::path::Component::Prefix(_))){return Err("잘못된 백업 경로".into());}
        let target=to.join(name);if let Some(p)=target.parent(){fs::create_dir_all(p).map_err(err)?;}fs::copy(from.join(name),target).map_err(err)?;
    }Ok(())
}
fn state(dir:&Path,phase:&str,message:&str)->Result<()> {storage::write_json(&dir.join("result.json"),&json!({"phase":phase,"message":message,"updatedAt":app_services::iso(),"pid":std::process::id()}))}
fn start(exe:&Path,args:&[String])->Result<std::process::Child>{Command::new(exe).args(args).creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(err)}
fn terminate(child:&mut std::process::Child){let _=child.kill();let _=child.wait();}
pub fn protect_directory(path:&Path)->Result<()>{
    let tool=PathBuf::from(std::env::var_os("SystemRoot").ok_or("SystemRoot 누락")?).join("System32/icacls.exe");
    let status=Command::new(tool).arg(path).args(["/setintegritylevel","(OI)(CI)H","/C","/Q"]).creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(err)?;
    if !status.success(){return Err("업데이트 작업 폴더를 보호하지 못했습니다".into());}Ok(())
}
fn set_file_integrity(path:&Path,level:&str)->Result<()>{
    let tool=PathBuf::from(std::env::var_os("SystemRoot").ok_or("SystemRoot 누락")?).join("System32/icacls.exe");
    let status=Command::new(tool).arg(path).args(["/setintegritylevel",level,"/C","/Q"]).creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(err)?;
    if !status.success(){return Err(format!("파일 무결성 수준 설정 실패: {status}"));}Ok(())
}
fn require_high_integrity(path:&Path)->Result<()>{
    let script=format!("$ErrorActionPreference='Stop';(Get-Acl -LiteralPath {}).Sddl",quote(&path.to_string_lossy()));
    let sddl=powershell::run(&script,10000,32768)?;
    if !sddl.contains("(ML;")||!sddl.contains(";;;HI)"){return Err("신뢰할 수 없는 업데이트 작업 파일입니다".into());}Ok(())
}
fn reject_reparse(path:&Path)->Result<()>{
    let mut current=Some(path);
    while let Some(item)=current {
        if item.exists()&&fs::symlink_metadata(item).map_err(err)?.file_attributes()&0x400!=0{return Err("재분석 지점을 통한 업데이트 경로는 지원하지 않습니다".into());}
        current=item.parent();
    }
    Ok(())
}
fn trusted_job(dir:&Path,files:&[&str])->Result<()>{
    let root=protected_cache_root()?;
    contained(dir,&root)?;
    let name=dir.file_name().and_then(|name|name.to_str()).ok_or("업데이트 작업 이름 누락")?;
    let id=name.strip_prefix("job-").ok_or("잘못된 업데이트 작업 이름")?;
    uuid::Uuid::parse_str(id).map_err(|_|"잘못된 업데이트 작업 식별자".to_string())?;
    reject_reparse(dir)?;
    require_high_integrity(dir)?;
    for name in files {
        let path=dir.join(name);
        if !path.exists(){return Err(format!("업데이트 작업 파일 누락: {name}"));}
        reject_reparse(&path)?;
        require_high_integrity(&path)?;
    }
    Ok(())
}
fn wide_path(path:&Path)->Vec<u16>{path.as_os_str().encode_wide().chain(Some(0)).collect()}
fn lock_directory(path:&Path)->Result<PathGuard>{
    let handle=unsafe{CreateFileW(wide_path(path).as_ptr(),FILE_READ_ATTRIBUTES,FILE_SHARE_READ|FILE_SHARE_WRITE,std::ptr::null(),OPEN_EXISTING,FILE_FLAG_BACKUP_SEMANTICS,std::ptr::null_mut())};
    if handle==INVALID_HANDLE_VALUE{return Err(std::io::Error::last_os_error().to_string());}
    Ok(PathGuard(handle))
}
pub fn protected_cache_root()->Result<PathBuf>{
    let base=PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA 누락")?);
    let canonical_base=base.canonicalize().map_err(err)?;
    let mut current=base;
    for name in ["NogiremUpdater","updates"]{
        current.push(name);
        match fs::create_dir(&current){Ok(())=>{},Err(error)if error.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(error)=>return Err(error.to_string())}
        if fs::symlink_metadata(&current).map_err(err)?.file_attributes()&0x400!=0{return Err("업데이트 캐시 경로에 연결된 외부 폴더가 있습니다".into());}
        protect_directory(&current)?;
        if fs::symlink_metadata(&current).map_err(err)?.file_attributes()&0x400!=0||!current.canonicalize().map_err(err)?.starts_with(&canonical_base){return Err("업데이트 캐시 보호 경로가 변경되었습니다".into());}
    }
    if !same(&current,&updater::cache_root()?){return Err("업데이트 캐시 경로가 일치하지 않습니다".into());}
    Ok(current)
}
fn write_migration_status(path:&Path,attempt:&str,phase:&str,percent:f64,error:Option<&str>)->Result<()>{
    uuid::Uuid::parse_str(attempt).map_err(|_|"잘못된 전환 시도 식별자")?;
    storage::write_json(path,&json!({"attempt":attempt,"phase":phase,"percent":percent,"error":error,"updatedAt":app_services::iso(),"pid":std::process::id()}))
}
pub fn migration_status(attempt:&str,phase:&str,percent:f64,error:Option<&str>)->Result<()>{
    write_migration_status(&protected_cache_root()?.join("migration-status.json"),attempt,phase,percent,error)
}
fn wait_exit(pids:&[u32],timeout:u64)->Result<()> {let end=Instant::now()+Duration::from_secs(timeout);while pids.iter().any(|p|process::alive(*p)){if Instant::now()>end{return Err("앱 종료를 기다리는 시간이 초과되었습니다".into());}std::thread::sleep(Duration::from_millis(100));}Ok(())}
fn supported_migration(version:&str)->Result<bool>{updater::newer(version,"0.4.0")}
fn stop_app(user:&Path,pid:u32)->Result<()> {
    if !process::alive(pid){return Ok(());}
    fs::create_dir_all(user.join("instance")).map_err(err)?;
    let deadline=Instant::now()+Duration::from_secs(90);
    while process::alive(pid) {
        fs::write(user.join("instance/installer-close-request"),format!("updater:{}",uuid::Uuid::new_v4())).map_err(err)?;
        let retry=Instant::now()+Duration::from_secs(1);
        while process::alive(pid)&&Instant::now()<retry{std::thread::sleep(Duration::from_millis(100));}
        if Instant::now()>=deadline{return Err("앱 종료를 기다리는 시간이 초과되었습니다".into());}
    }
    Ok(())
}
fn launch_healthy(dir:&Path,job:&Job,manifest:&Manifest)->Result<u32> {
    let _=fs::remove_file(dir.join("healthy.json"));
    let executable=job.portable_exe.as_ref().cloned().unwrap_or_else(||job.install_dir.join("nogirem.exe"));
    let child=start(&executable,&[format!("--update-receipt={}",dir.display())])?;
    let end=Instant::now()+Duration::from_secs(120);
    while Instant::now()<end {
        let value=storage::read_json(&dir.join("healthy.json")).ok().flatten().unwrap_or(Value::Null);
        if value["nonce"]==job.nonce && value["version"]==manifest.version && value["ready"]==true{return Ok(value["pid"].as_u64().unwrap_or(child.id() as u64) as u32);}
        if !job.portable&& !process::alive(child.id()){break;}
        std::thread::sleep(Duration::from_millis(150));
    }
    stop_app(&job.user_dir,child.id())?;
    Err("신규 앱의 UI·백엔드 시작 확인에 실패했습니다".into())
}
pub fn mark_healthy(args:&[String])->Result<()> {
    let Some(path)=args.iter().find_map(|a|a.strip_prefix("--update-receipt=")) else{return Ok(());};
    let dir=Path::new(path);contained(dir,&updater::cache_root()?)?;
    let job:Job=serde_json::from_slice(&fs::read(dir.join("job.json")).map_err(err)?).map_err(err)?;
    if job.nonce.len()!=36 || !same(&job.user_dir,&expected_user(&job)?){return Err("잘못된 업데이트 시작 확인 요청".into());}
    storage::write_json(&dir.join("healthy.json"),&json!({"ready":true,"nonce":job.nonce,"version":env!("CARGO_PKG_VERSION"),"pid":std::process::id()}))
}
fn abandoned_job(phase:&str,alive:bool,stale:bool,recovery:bool)->bool{stale&&!alive&&!recovery&&matches!(phase,"preparing"|"rolled-back")}
pub fn cleanup_cache() -> Result<()> {
    let root=protected_cache_root()?;
    if !root.exists(){return Ok(());}
    for entry in fs::read_dir(&root).map_err(err)? {
        let dir=entry.map_err(err)?.path();let name=dir.file_name().unwrap_or_default().to_string_lossy();
        let id=name.strip_prefix("job-").or_else(||name.strip_prefix("download-"));
        if !id.is_some_and(|id|uuid::Uuid::parse_str(id).is_ok()){continue;}
        if fs::symlink_metadata(&dir).map_err(err)?.file_attributes()&0x400!=0 {continue;}
        let completed=storage::read_json(&dir.join("result.json")).ok().flatten().unwrap_or(Value::Null);
        let download=name.starts_with("download-") && dir.join("owned-by-updater").is_file();
        let alive=completed["pid"].as_u64().is_some_and(|p|process::alive(p as u32));
        let modified=fs::metadata(dir.join("result.json")).or_else(|_|fs::metadata(&dir)).and_then(|metadata|metadata.modified()).unwrap_or(SystemTime::now());
        let stale=SystemTime::now().duration_since(modified).unwrap_or_default()>=Duration::from_secs(24*60*60);
        let phase=completed["phase"].as_str().unwrap_or("");
        let finished=phase=="complete"&&!alive;
        let abandoned=abandoned_job(phase,alive,stale,dir.join("portable-recovery.json").exists());
        if !download && !finished && !abandoned {continue;}
        contained(&dir,&root)?;
        let Ok(entries)=files(&dir) else {continue;};
        for file in entries {let path=dir.join(file);contained(&path,&root)?;fs::remove_file(path).map_err(err)?;}
        fn empty_dirs(path:&Path,root:&Path)->Result<()> {
            for entry in fs::read_dir(path).map_err(err)? {let p=entry.map_err(err)?.path();contained(&p,root)?;if p.is_dir(){empty_dirs(&p,root)?;}}
            contained(path,root)?;fs::remove_dir(path).map_err(err)
        }
        empty_dirs(&dir,&root)?;
    }
    Ok(())
}
pub fn prepare(exe:&Path,user:&Path,envelope:&[u8],payload:&Path,pids:Vec<u32>,portable:bool)->Result<PathBuf>{
    let m=updater::verify_envelope_for(envelope,&updater::config(),portable)?;updater::verify_file(payload,&m)?;
    let install_dir=if portable {
        let dir=exe.parent().ok_or("포터블 실행 경로 누락")?.to_owned();
        if !exe.is_file()||!exe.extension().is_some_and(|extension|extension.eq_ignore_ascii_case("exe")){return Err("포터블 실행 파일을 확인하지 못했습니다".into());}
        dir
    } else {
        let dir=installation()?;
        if !same(exe,&dir.join("nogirem.exe")) {return Err("설치된 앱에서 업데이트를 실행해 주세요".into());}
        dir
    };
    let dir=protected_cache_root()?.join(format!("job-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&dir).map_err(err)?;protect_directory(&dir)?;
    let startup=app_services::startup(exe,true,None)?["enabled"]==true;
    let job=Job{nonce:uuid::Uuid::new_v4().to_string(),parent_pids:pids,install_dir,user_dir:user.into(),startup,legacy_exe:None,portable,portable_exe:portable.then(||exe.to_owned())};
    fs::write(dir.join("update.json"),envelope).map_err(err)?;
    fs::copy(payload,dir.join(if portable{"portable.exe"}else{"installer.exe"})).map_err(err)?;
    fs::copy(std::env::current_exe().map_err(err)?,dir.join("updater.exe")).map_err(err)?;
    storage::write_json(&dir.join("job.json"),&serde_json::to_value(job).map_err(err)?)?;
    start(&dir.join("updater.exe"),&["--apply-update".into(),dir.to_string_lossy().into()])?;
    let deadline=Instant::now()+Duration::from_secs(10);
    while !dir.join("helper-ready").exists(){if Instant::now()>deadline{return Err("업데이트 설치 프로세스를 시작하지 못했습니다".into());}std::thread::sleep(Duration::from_millis(50));}
    Ok(dir)
}
fn shortcuts(exe:&Path)->Result<()> {
    let path=quote(&exe.to_string_lossy());
    powershell::run(&format!(r#"$ErrorActionPreference='Stop'
$s=New-Object -ComObject WScript.Shell
foreach($base in @([Environment]::GetFolderPath('Desktop'),([Environment]::GetFolderPath('StartMenu')+'\Programs'))){{
 $link=$s.CreateShortcut((Join-Path $base '마비노기 렘 부스터.lnk'));$link.TargetPath={path};$link.WorkingDirectory=Split-Path {path};$link.IconLocation={path}+',0';$link.Save()
}}"#),15000,65536)?;Ok(())
}
fn legacy_uninstaller(exe:&Path)->Result<PathBuf> {
    let directory=exe.parent().ok_or("기존 설치 경로 누락")?;
    if exe.file_name().and_then(|s|s.to_str())!=Some("마비노기 렘 부스터.exe") || !exe.is_file(){return Err("기존 Electron 설치를 확인하지 못했습니다".into());}
    let rows=powershell::json(r#"$ErrorActionPreference='Stop'
@('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*')|ForEach-Object {Get-ItemProperty $_ -ErrorAction SilentlyContinue}|Where-Object {$_.DisplayName -like '마비노기 렘 부스터*'}|Select-Object DisplayIcon,InstallLocation,UninstallString|ConvertTo-Json -Compress"#,15000)?;
    let entries=if let Some(a)=rows.as_array(){a.clone()}else{vec![rows]};
    for row in entries {let text=row["UninstallString"].as_str().unwrap_or("");let executable=if let Some(t)=text.strip_prefix('"'){t.split('"').next().unwrap_or("")}else{text.split(" /" ).next().unwrap_or("")};
        let p=PathBuf::from(executable);if p.parent().is_some_and(|p|same(p,directory)) && p.file_name().is_some_and(|n|n.to_string_lossy().starts_with("Uninstall")) && p.is_file(){return Ok(p);}
    }Err("기존 Electron 제거 등록을 찾지 못했습니다".into())
}
fn remove_legacy(dir:&Path,exe:&Path)->Result<()> {
    state(dir,"locating-legacy-uninstaller","")?;
    let uninstall=legacy_uninstaller(exe)?;
    let copy=dir.join("uninstall-electron.exe");fs::copy(uninstall,&copy).map_err(err)?;
    let mut child=start(&copy,&["/S".into(),format!("_?={}",exe.parent().unwrap().display())])?;
    state(dir,"waiting-legacy-uninstaller",&child.id().to_string())?;
    let deadline=Instant::now()+Duration::from_secs(60);
    let status=loop {
        if let Some(status)=child.try_wait().map_err(err)? {break status;}
        if Instant::now()>=deadline {return Err(format!("기존 앱 제거 프로세스 종료 지연 (PID {}). 신규 앱은 설치되어 있습니다",child.id()));}
        std::thread::sleep(Duration::from_millis(100));
    };
    if !status.success() || exe.exists(){return Err("기존 Electron 제거가 완료되지 않았습니다. 신규 앱은 설치되어 있습니다".into());}Ok(())
}
fn settings_files(root:&Path)->Result<Vec<PathBuf>> {
    let mut out=Vec::new();
    if !root.exists(){return Ok(out);}
    for entry in fs::read_dir(root).map_err(err)? {
        let p=entry.map_err(err)?.path();let m=fs::symlink_metadata(&p).map_err(err)?;
        if m.file_attributes()&0x400!=0 {continue;}
        if m.is_file() && p.extension().is_some_and(|e|e=="json") {out.push(p.strip_prefix(root).map_err(err)?.into());}
        // Only application-owned settings directories, never WebView profiles or recordings.
        if m.is_dir() && p.file_name().and_then(|n|n.to_str()).is_some_and(|n|matches!(n,"game"|"creator"|"network"|"affinity"|"blackbox"|"input-guard"|"turbo-key"|"vulkan"|"memory")) {
            for entry in fs::read_dir(&p).map_err(err)? {
                let f=entry.map_err(err)?.path();let m=fs::symlink_metadata(&f).map_err(err)?;
                if m.file_attributes()&0x400==0 && m.is_file() && f.extension().is_some_and(|e|e=="json") {out.push(f.strip_prefix(root).map_err(err)?.into());}
            }
        }
    }
    Ok(out)
}
fn registry_backup(dir:&Path)->Result<()> {
    let dest=quote(&dir.join("previous-registration.reg").to_string_lossy());
    powershell::run(&format!(r#"$ErrorActionPreference='Stop'
if(Test-Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus') {{
 & reg.exe export 'HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus' {dest} /y /reg:64 | Out-Null
 if($LASTEXITCODE -ne 0) {{throw 'Failed to back up installation registration'}}
}}"#),15000,65536)?;Ok(())
}
fn registry_restore(dir:&Path)->Result<()> {
    let backup=dir.join("previous-registration.reg");
    let script=if backup.exists(){format!("& reg.exe import {} /reg:64 | Out-Null; if($LASTEXITCODE -ne 0) {{throw 'Failed to restore installation registration'}}",quote(&backup.to_string_lossy()))}
        else{r"Remove-Item -LiteralPath 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus' -ErrorAction SilentlyContinue".into()};
    powershell::run(&script,15000,65536)?;Ok(())
}
fn reader_sha256(reader:&mut impl Read)->Result<String>{let mut hash=Sha256::new();let mut chunk=[0u8;65536];loop{let n=reader.read(&mut chunk).map_err(err)?;if n==0{break;}hash.update(&chunk[..n]);}Ok(format!("{:x}",hash.finalize()))}
fn file_sha256(path:&Path)->Result<String>{reader_sha256(&mut File::open(path).map_err(err)?)}
fn replace_file(source:&Path,target:&Path,expected_hash:&str)->Result<File>{
    if !file_sha256(source)?.eq_ignore_ascii_case(expected_hash){return Err("교체 원본 파일 무결성 확인 실패".into());}
    let parent=target.parent().ok_or("포터블 대상 상위 경로 누락")?;
    reject_reparse(parent)?;
    if target.exists(){reject_reparse(target)?;}
    let canonical_parent=parent.canonicalize().map_err(err)?;
    let _parent_guard=lock_directory(parent)?;
    let staging=parent.join(format!(".nogirem-stage-{}",uuid::Uuid::new_v4()));
    fs::create_dir(&staging).map_err(err)?;
    let _staging_cleanup=RemoveDirectory(staging.clone());
    let stage_guard=lock_directory(&staging)?;
    reject_reparse(&staging)?;
    protect_directory(&staging)?;
    require_high_integrity(&staging)?;
    let temporary=staging.join("payload.exe");
    let mut output=OpenOptions::new().write(true).create_new(true).share_mode(FILE_SHARE_READ).open(&temporary).map_err(err)?;
    let copied=(||->Result<()> {
        set_file_integrity(&temporary,"H")?;
        require_high_integrity(&temporary)?;
        let mut input=File::open(source).map_err(err)?;
        std::io::copy(&mut input,&mut output).map_err(err)?;
        output.flush().map_err(err)?;
        output.sync_all().map_err(err)?;
        Ok(())
    })();
    drop(output);
    if let Err(error)=copied{return Err(error);}
    if !file_sha256(&temporary)?.eq_ignore_ascii_case(expected_hash){return Err("교체 임시 파일 무결성 확인 실패".into());}
    let moved=unsafe{windows_sys::Win32::Storage::FileSystem::MoveFileExW(wide_path(&temporary).as_ptr(),wide_path(target).as_ptr(),0x1|0x8)};
    drop(stage_guard);
    if moved==0{return Err(std::io::Error::last_os_error().to_string());}
    if parent.canonicalize().map_err(err)?!=canonical_parent{return Err("포터블 대상 경로가 교체 중 변경되었습니다".into());}
    reject_reparse(target)?;
    require_high_integrity(target)?;
    let mut guard=OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(target).map_err(err)?;
    if !reader_sha256(&mut guard)?.eq_ignore_ascii_case(expected_hash){return Err("교체된 포터블 파일 무결성 확인 실패".into());}
    Ok(guard)
}
fn apply_portable(dir:&Path,job:&Job,manifest:&Manifest)->Result<()>{
    let payload=dir.join("portable.exe");updater::verify_file(&payload,manifest)?;
    let target=job.portable_exe.as_ref().ok_or("포터블 실행 파일 경로 누락")?;
    if !target.is_file()||target.parent().is_none_or(|parent|!same(parent,&job.install_dir)){return Err("포터블 업데이트 대상이 올바르지 않습니다".into());}
    let backup=dir.join("previous-portable.exe");fs::copy(target,&backup).map_err(err)?;
    let backup_hash=file_sha256(&backup)?;
    storage::write_json(&dir.join("portable-recovery.json"),&json!({"source":target,"sha256":backup_hash}))?;
    state(dir,"installing",&manifest.version)?;
    let target_guard=replace_file(&payload,target,&manifest.sha256)?;
    if let Err(error)=launch_healthy(dir,job,manifest){
        drop(target_guard);
        let rollback_guard=replace_file(&backup,target,&backup_hash)?;
        let _=set_file_integrity(target,"M");
        drop(rollback_guard);
        state(dir,"rolled-back",&error)?;return Err(error);
    }
    let _=set_file_integrity(target,"M");
    drop(target_guard);
    fs::rename(dir.join("portable-recovery.json"),dir.join("portable-complete.json")).map_err(err)?;
    let _=app_services::startup(target,true,Some(job.startup));
    let _=state(dir,"complete",&manifest.version);Ok(())
}
pub fn start_interrupted_portable_recovery(source:&Path,launcher_pid:u32)->Result<bool>{
    let root=updater::cache_root()?;
    if !root.exists(){return Ok(false);}
    for entry in fs::read_dir(&root).map_err(err)?{
        let dir=entry.map_err(err)?.path();
        if !dir.file_name().unwrap_or_default().to_string_lossy().starts_with("job-")||!dir.join("portable-recovery.json").is_file(){continue;}
        if trusted_job(&dir,&["job.json","result.json","update.json","portable-recovery.json","previous-portable.exe"]).is_err(){continue;}
        let job:Job=serde_json::from_slice(&fs::read(dir.join("job.json")).map_err(err)?).map_err(err)?;
        let status=storage::read_json(&dir.join("result.json")).map_err(err)?.unwrap_or(Value::Null);
        if !job.portable||job.portable_exe.as_ref().is_none_or(|path|!same(path,source))||status["phase"]!="installing"{continue;}
        if status["pid"].as_u64().is_some_and(|pid|process::alive(pid as u32)){return Ok(true);}
        let helper=dir.join(format!("recovery-{}.exe",uuid::Uuid::new_v4()));fs::copy(std::env::current_exe().map_err(err)?,&helper).map_err(err)?;
        require_high_integrity(&helper)?;
        start(&helper,&["--recover-portable-exe".into(),dir.to_string_lossy().into(),source.to_string_lossy().into(),launcher_pid.to_string()])?;
        return Ok(true);
    }
    Ok(false)
}
pub fn recover_portable_exe(dir:&Path,source:&Path,launcher_pid:u32)->Result<()>{
    trusted_job(dir,&["job.json","result.json","update.json","portable-recovery.json","previous-portable.exe"])?;
    if launcher_pid==0||!same(Path::new(&process::path(launcher_pid)?),source){return Err("포터블 런처 프로세스를 확인하지 못했습니다".into());}
    wait_exit(&[launcher_pid],120)?;
    let _guard=migration_guard()?;
    let job:Job=serde_json::from_slice(&fs::read(dir.join("job.json")).map_err(err)?).map_err(err)?;
    let status=storage::read_json(&dir.join("result.json")).map_err(err)?.ok_or("포터블 업데이트 상태 누락")?;
    if status["phase"]!="installing"||status["pid"].as_u64().is_some_and(|pid|process::alive(pid as u32)){return Err("복구 가능한 포터블 업데이트 상태가 아닙니다".into());}
    let recovery=storage::read_json(&dir.join("portable-recovery.json")).map_err(err)?.ok_or("포터블 복구 정보 누락")?;
    let backup=dir.join("previous-portable.exe");
    let backup_hash=recovery["sha256"].as_str().ok_or("포터블 복구 해시 누락")?;
    if !job.portable||job.portable_exe.as_ref().is_none_or(|path|!same(path,source))||recovery["source"].as_str().is_none_or(|path|!same(Path::new(path),source))||!file_sha256(&backup)?.eq_ignore_ascii_case(backup_hash){return Err("포터블 복구 정보를 확인하지 못했습니다".into());}
    let guard=replace_file(&backup,source,backup_hash)?;
    set_file_integrity(source,"M")?;
    drop(guard);
    fs::rename(dir.join("portable-recovery.json"),dir.join("portable-complete.json")).map_err(err)?;
    state(dir,"rolled-back","중단된 포터블 업데이트를 복구했습니다")?;
    Ok(())
}
pub fn apply(dir:&Path)->Result<()> {
    trusted_job(dir,&["job.json","result.json","update.json"])?;
    let job:Job=serde_json::from_slice(&fs::read(dir.join("job.json")).map_err(err)?).map_err(err)?;
    trusted_job(dir,&[if job.portable{"portable.exe"}else{"installer.exe"}])?;
    let valid_install=if job.portable{job.portable_exe.as_ref().is_some_and(|exe|exe.is_file()&&std::env::var_os("NOGIREM_PORTABLE_SOURCE").is_some_and(|source|same(Path::new(&source),exe)))}else{same(&job.install_dir,&installation()?)};
    if !valid_install || !same(&job.user_dir,&expected_user(&job)?) || uuid::Uuid::parse_str(&job.nonce).is_err(){return Err("잘못된 설치 작업 경로".into());}
    let manifest=updater::verify_envelope_for(&fs::read(dir.join("update.json")).map_err(err)?,&updater::config(),job.portable)?;
    updater::verify_file(&dir.join(if job.portable{"portable.exe"}else{"installer.exe"}),&manifest)?;
    state(dir,"preparing",&manifest.version)?;
    fs::write(dir.join("helper-ready"),b"ready").map_err(err)?;
    wait_exit(&job.parent_pids,120)?;
    let _guard=migration_guard()?;
    // Require a commit marker: if the running app could not flush/stop safely, leave it alone.
    if job.legacy_exe.is_none() {
        let commit=dir.join("commit");
        if !commit.exists(){return Err("앱에서 업데이트 설치를 취소했습니다".into());}
        reject_reparse(&commit)?;
        require_high_integrity(&commit)?;
    }
    if job.portable{return apply_portable(dir,&job,&manifest);}
    let installed=files(&job.install_dir)?;
    let backup=dir.join("previous-install");copy_files(&job.install_dir,&backup,&installed)?;
    // Back up user JSON only. Recordings live outside userData and are never copied/deleted.
    let settings=settings_files(&job.user_dir)?;
    registry_backup(dir)?;
    copy_files(&job.user_dir,&dir.join("previous-settings"),&settings)?;
    state(dir,"installing",&manifest.version)?;
    let attempt=(||{
        let status=start(&dir.join("installer.exe"),&["/S".into()])?.wait().map_err(err)?;
        if !status.success(){return Err(format!("설치 프로그램 실패: {status}"));}
        launch_healthy(dir,&job,&manifest)
    })();
    if let Err(error)=attempt {
        copy_files(&backup,&job.install_dir,&installed)?;
        copy_files(&dir.join("previous-settings"),&job.user_dir,&settings)?;
        registry_restore(dir)?;
        let fallback=job.legacy_exe.clone().unwrap_or_else(||job.install_dir.join("nogirem.exe"));
        if fallback.exists(){let _=app_services::startup(&fallback,true,Some(job.startup));let _=shortcuts(&fallback);let _=start(&fallback,&["--migration-retry-suppressed".into()]);}
        state(dir,"rolled-back",&error)?;return Err(error);
    }
    // A healthy Rust app is now running. Only now may the old Electron install be removed.
    state(dir,"removing-legacy",&manifest.version)?;
    let cleanup=job.legacy_exe.as_ref().map(|old|remove_legacy(dir,old)).transpose();
    let new_exe=job.install_dir.join("nogirem.exe");
    state(dir,"restoring-startup",&manifest.version)?;
    app_services::startup(&new_exe,true,Some(job.startup))?;
    state(dir,"restoring-shortcuts",&manifest.version)?;
    shortcuts(&new_exe)?;
    if let Err(e)=cleanup {state(dir,"cleanup-needed",&e)?;return Err(e);}
    state(dir,"complete",&manifest.version)?;Ok(())
}
pub fn recover_preinstall(dir:&Path) {
    let recover=(||->Result<()> {
        contained(dir,&updater::cache_root()?)?;
        let job:Job=serde_json::from_slice(&fs::read(dir.join("job.json")).map_err(err)?).map_err(err)?;
        let status=storage::read_json(&dir.join("result.json")).map_err(err)?.unwrap_or(Value::Null);
        if status["phase"]!="preparing" || job.parent_pids.iter().any(|p|process::alive(*p)) {return Ok(());}
        let valid_install=if job.portable{job.portable_exe.as_ref().is_some_and(|path|path.is_file())}else{same(&job.install_dir,&installation()?)};
        if !valid_install || !same(&job.user_dir,&expected_user(&job)?){return Err("작업 경로 불일치".into());}
        if job.portable{return Ok(());}
        let target=job.legacy_exe.as_ref().unwrap_or(&job.install_dir).to_owned();
        let exe=if job.legacy_exe.is_some(){legacy_uninstaller(&target)?;target}else{target.join("nogirem.exe")};
        start(&exe,&["--migration-retry-suppressed".into()])?;
        Ok(())
    })();let _=recover;
}
pub fn migrate(legacy:&Path,pid:u32,attempt:Option<&str>)->Result<()> {
    let _guard=migration_guard()?;
    let status_path=if attempt.is_some(){Some(protected_cache_root()?.join("migration-status.json"))}else{None};
    if let (Some(attempt),Some(path))=(attempt,status_path.as_ref()){write_migration_status(path,attempt,"checking",0.,None)?;}
    legacy_uninstaller(legacy)?;
    if !same(Path::new(&process::path(pid)?),legacy){return Err("기존 앱 프로세스가 일치하지 않습니다".into());}
    let (m,envelope)=updater::fetch_manifest()?;
    if !supported_migration(&m.version)? {return Err("Rust 0.4.1 이상의 전환 릴리스가 아직 준비되지 않았습니다".into());}
    let dir=protected_cache_root()?.join(format!("job-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&dir).map_err(err)?;protect_directory(&dir)?;
    state(&dir,"downloading",&m.version)?;
    if let (Some(attempt),Some(path))=(attempt,status_path.as_ref()){write_migration_status(path,attempt,"downloading",0.,None)?;}
    let progress_at=std::sync::Mutex::new(Instant::now()-Duration::from_secs(1));
    updater::download(&m,&dir.join("installer.exe"),&|n|{
        if let (Some(attempt),Some(path))=(attempt,status_path.as_ref()) {
            let mut last=progress_at.lock().unwrap();
            if last.elapsed()>=Duration::from_millis(200)||n==m.size {
                *last=Instant::now();
                let _=write_migration_status(path,attempt,"downloading",(n as f64*100./m.size as f64).min(99.9),None);
            }
        }
    },&std::sync::atomic::AtomicBool::new(false))?;
    if let (Some(attempt),Some(path))=(attempt,status_path.as_ref()){write_migration_status(path,attempt,"installing",99.9,None)?;}
    let job=Job{nonce:uuid::Uuid::new_v4().to_string(),parent_pids:vec![pid],install_dir:installation()?,user_dir:user_dir()?,startup:app_services::startup(legacy,true,None)?["enabled"]==true,legacy_exe:Some(legacy.into()),portable:false,portable_exe:None};
    fs::write(dir.join("update.json"),envelope).map_err(err)?;
    storage::write_json(&dir.join("job.json"),&serde_json::to_value(&job).map_err(err)?)?;
    // Electron 제거기가 번들 helper를 지울 수 있도록 보호된 준비 사본에서 실행한다.
    let helper=dir.join("updater.exe");fs::copy(std::env::current_exe().map_err(err)?,&helper).map_err(err)?;
    let mut apply=start(&helper,&["--apply-update".into(),dir.to_string_lossy().into()])?;
    let ready_deadline=Instant::now()+Duration::from_secs(10);
    while !dir.join("helper-ready").is_file() {
        match apply.try_wait() {
            Ok(Some(status))=>return Err(format!("업데이트 적용 helper가 준비 전에 종료되었습니다: {status}")),
            Ok(None)=>{},
            Err(error)=>{terminate(&mut apply);return Err(error.to_string());}
        }
        if Instant::now()>=ready_deadline {
            terminate(&mut apply);
            return Err("업데이트 적용 helper가 준비되지 않았습니다".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if let (Some(attempt),Some(path))=(attempt,status_path.as_ref()){if let Err(error)=write_migration_status(path,attempt,"handoff",99.9,None){terminate(&mut apply);return Err(error);}}
    if let Err(error)=stop_app(&job.user_dir,pid){
        terminate(&mut apply);
        return Err(error);
    }
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn migration_rejects_versions_before_0_4_1(){assert!(!supported_migration("0.4.0").unwrap());assert!(supported_migration("0.4.1").unwrap());assert!(supported_migration("0.5.0").unwrap());}
    #[test]fn backup_restore_preserves_unrelated_files(){let root=std::env::temp_dir().join(format!("nogirem-update-test-{}",uuid::Uuid::new_v4()));let old=root.join("old");let backup=root.join("backup");fs::create_dir_all(&old).unwrap();fs::write(old.join("app.exe"),b"old").unwrap();let list=files(&old).unwrap();copy_files(&old,&backup,&list).unwrap();fs::write(old.join("app.exe"),b"new").unwrap();fs::write(old.join("user.txt"),b"keep").unwrap();copy_files(&backup,&old,&list).unwrap();assert_eq!(fs::read(old.join("app.exe")).unwrap(),b"old");assert_eq!(fs::read(old.join("user.txt")).unwrap(),b"keep");assert!(copy_files(&old,&backup,&[PathBuf::from("../outside")]).is_err());for dir in [&old,&backup]{for p in files(dir).unwrap(){fs::remove_file(dir.join(p)).unwrap();}fs::remove_dir(dir).unwrap();}fs::remove_dir(root).unwrap();}
    #[test]fn migration_guard_prevents_concurrent_helpers(){let first=migration_guard().unwrap();assert!(migration_guard().is_err());drop(first);assert!(migration_guard().is_ok());}
    #[test]fn abandoned_cache_requires_age_and_preserves_recovery(){assert!(abandoned_job("preparing",false,true,false));assert!(abandoned_job("rolled-back",false,true,false));assert!(!abandoned_job("preparing",false,false,false));assert!(!abandoned_job("preparing",true,true,false));assert!(!abandoned_job("installing",false,true,false));assert!(!abandoned_job("rolled-back",false,true,true));}
}
