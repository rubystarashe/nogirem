use crate::{Result, http};
use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::Duration};

pub const STARTUP_CHECK_DELAY_MS:u64=3_000;
pub const CHECK_INTERVAL_MS:u64=4*60*60*1_000;
pub const COMPLETION_DELAY_MS:u64=350;
const MAX_INSTALLER: u64 = 512 * 1024 * 1024;
#[derive(Clone, Deserialize)]
pub struct Config { pub repository: String, pub public_key: String }
pub fn config() -> Config {
    let v: Value = serde_json::from_str(include_str!("../../../build-config/update.json")).expect("embedded update configuration");
    Config { repository:v["repository"].as_str().unwrap().into(), public_key:v["publicKey"].as_str().unwrap().into() }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub version: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)] pub notes: String,
}
pub fn verify_envelope_for(bytes: &[u8], cfg: &Config, portable: bool) -> Result<Manifest> {
    if bytes.len()>64*1024 {return Err("업데이트 정보 크기 초과".into());}
    let envelope: Value=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    let payload=envelope["payload"].as_str().ok_or("서명된 업데이트 본문 누락")?;
    let signature=STANDARD.decode(envelope["signature"].as_str().ok_or("업데이트 서명 누락")?).map_err(|e|e.to_string())?;
    let key: [u8;32]=STANDARD.decode(&cfg.public_key).map_err(|e|e.to_string())?.try_into().map_err(|_|"공개키 길이 오류")?;
    VerifyingKey::from_bytes(&key).map_err(|e|e.to_string())?.verify_strict(payload.as_bytes(),&Signature::from_slice(&signature).map_err(|e|e.to_string())?).map_err(|_|"업데이트 서명 검증 실패")?;
    let m: Manifest=serde_json::from_str(payload).map_err(|e|e.to_string())?;
    let version=semver::Version::parse(&m.version).map_err(|_|"업데이트 버전 형식 오류")?;
    if m.schema_version!=1 || !version.pre.is_empty() || !version.build.is_empty() || m.size==0 || m.size>MAX_INSTALLER || m.sha256.len()!=64 || !m.sha256.bytes().all(|c|c.is_ascii_hexdigit()) {return Err("업데이트 정보 형식 오류".into());}
    let asset=if portable {format!("nogirem-dioxus-portable-{}.exe",m.version)} else {format!("nogirem-dioxus-setup-{}.exe",m.version)};
    let expected=format!("https://github.com/{}/releases/download/v{}/{}",cfg.repository,m.version,asset);
    if m.url!=expected {return Err("허용되지 않은 업데이트 다운로드 주소".into());}
    Ok(m)
}
pub fn verify_envelope(bytes: &[u8], cfg: &Config) -> Result<Manifest> {verify_envelope_for(bytes,cfg,false)}
pub fn newer(candidate:&str,current:&str)->Result<bool>{
    Ok(semver::Version::parse(candidate).map_err(|e|e.to_string())?>semver::Version::parse(current).map_err(|e|e.to_string())?)
}
pub fn fetch_manifest_for(portable:bool)->Result<(Manifest,Vec<u8>)>{
    let cfg=config();
    let name=if portable{"portable-update.json"}else{"update.json"};
    let bytes=http::bytes(&format!("https://github.com/{}/releases/latest/download/{name}",cfg.repository),64*1024,15000,&[("Cache-Control","no-cache")])?;
    let m=verify_envelope_for(&bytes,&cfg,portable)?; Ok((m,bytes))
}
pub fn fetch_manifest()->Result<(Manifest,Vec<u8>)>{fetch_manifest_for(false)}
pub fn verify_file(path:&Path,m:&Manifest)->Result<()> {
    let mut file=fs::File::open(path).map_err(|e|e.to_string())?;
    if file.metadata().map_err(|e|e.to_string())?.len()!=m.size {return Err("설치파일 크기가 일치하지 않습니다".into());}
    let mut hash=Sha256::new();let mut chunk=[0u8;65536];
    loop {let n=file.read(&mut chunk).map_err(|e|e.to_string())?;if n==0{break;}hash.update(&chunk[..n]);}
    if !format!("{:x}",hash.finalize()).eq_ignore_ascii_case(&m.sha256){return Err("설치파일 해시 검증 실패".into());} Ok(())
}
fn write_download(mut input:impl Read,path:&Path,m:&Manifest,progress:&dyn Fn(u64),cancel:&AtomicBool)->Result<()> {
    let mut file=fs::OpenOptions::new().create_new(true).write(true).open(path).map_err(|e|e.to_string())?;
    let result=(||{
        let mut count=0;let mut hash=Sha256::new();let mut chunk=[0u8;65536];
        loop {
            if cancel.load(Ordering::SeqCst){return Err("업데이트 다운로드가 취소되었습니다".into());}
            let n=input.read(&mut chunk).map_err(|e|e.to_string())?;if n==0{break;}
            count+=n as u64;if count>m.size{return Err("설치파일이 예상 크기를 초과했습니다".into());}
            file.write_all(&chunk[..n]).map_err(|e|e.to_string())?;hash.update(&chunk[..n]);progress(count);
        }
        if count!=m.size || !format!("{:x}",hash.finalize()).eq_ignore_ascii_case(&m.sha256){return Err("다운로드가 불완전하거나 해시가 일치하지 않습니다".into());}
        file.sync_all().map_err(|e|e.to_string())
    })();
    drop(file);if result.is_err(){let _=fs::remove_file(path);}result
}
fn download_client(https_only:bool)->Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder().https_only(https_only).connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(5*60)).user_agent("nogirem-updater").build().map_err(|e|e.to_string())
}
fn download_with_client(client:&reqwest::blocking::Client,m:&Manifest,path:&Path,progress:&dyn Fn(u64),cancel:&AtomicBool)->Result<()> {
    let response=client.get(&m.url).send().map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;
    if response.content_length().is_some_and(|n|n!=m.size){return Err("서버의 설치파일 크기가 일치하지 않습니다".into());}
    write_download(response,path,m,progress,cancel)
}
pub fn download(m:&Manifest,path:&Path,progress:&dyn Fn(u64),cancel:&AtomicBool)->Result<()> {
    download_with_client(&download_client(true)?,m,path,progress,cancel)
}
pub fn cache_root()->Result<PathBuf>{
    Ok(PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA 누락")?).join("NogiremUpdater/updates"))
}
struct Data { state:Value, manifest:Option<Manifest>, envelope:Vec<u8>, file:Option<PathBuf> }
pub struct Updater { data:Mutex<Data>, busy:AtomicBool, cancel:AtomicBool, portable:bool, notify:Arc<dyn Fn(Value)+Send+Sync> }
impl Updater {
    pub fn new(portable:bool,notify:Arc<dyn Fn(Value)+Send+Sync>)->Arc<Self>{Arc::new(Self{data:Mutex::new(Data{state:json!({"phase":"idle","version":null,"percent":0,"error":null}),manifest:None,envelope:vec![],file:None}),busy:AtomicBool::new(false),cancel:AtomicBool::new(false),portable,notify})}
    pub fn state(&self)->Value{self.data.lock().unwrap().state.clone()}
    fn set(&self,phase:&str,percent:f64,error:Option<String>){
        let state={let mut d=self.data.lock().unwrap();let version=d.manifest.as_ref().map(|m|m.version.clone());let notes=d.manifest.as_ref().map(|m|m.notes.clone()).unwrap_or_default();d.state=json!({"phase":phase,"version":version,"percent":percent,"error":error,"notes":notes});d.state.clone()};
        (self.notify)(state);
    }
    pub fn check(self:&Arc<Self>)->Value {self.check_internal(false)}
    pub fn request(self:&Arc<Self>)->Result<Value> {
        match self.state()["phase"].as_str() {
            Some("available")=>self.start_download(),
            Some("downloading"|"downloaded"|"installing")=>Ok(self.state()),
            _=>Ok(self.check_internal(true)),
        }
    }
    fn check_internal(self:&Arc<Self>,auto_download:bool)->Value {
        if matches!(self.state()["phase"].as_str(),Some("available"|"downloading"|"downloaded"|"installing")) || self.busy.swap(true,Ordering::SeqCst){return self.state();}
        self.set("checking",0.,None);
        let this=self.clone();std::thread::spawn(move||{
            let result=(||{let (manifest,envelope)=fetch_manifest_for(this.portable)?;let update=newer(&manifest.version,env!("CARGO_PKG_VERSION"))?;
                {let mut d=this.data.lock().unwrap();d.manifest=if update{Some(manifest)}else{None};d.envelope=envelope;d.file=None;}
                this.set(if update{"available"}else{"idle"},0.,None);Ok::<_,String>(())})();
            if let Err(e)=result{this.set("error",0.,Some(e));}this.busy.store(false,Ordering::SeqCst);
            if auto_download && this.state()["phase"]=="available" {let _=this.start_download();}
        });self.state()
    }
    pub fn start_download(self:&Arc<Self>)->Result<Value>{
        if self.state()["phase"]!="available" || self.busy.swap(true,Ordering::SeqCst){return Ok(self.state());}
        let manifest=self.data.lock().unwrap().manifest.clone();
        let Some(m)=manifest else{self.busy.store(false,Ordering::SeqCst);return Err("먼저 업데이트를 확인해 주세요".into());};
        self.cancel.store(false,Ordering::SeqCst);self.set("downloading",0.,None);
        let this=self.clone();std::thread::spawn(move||{
            let result=(||{
                crate::update_install::cleanup_cache()?;
                let dir=crate::update_install::protected_cache_root()?.join(format!("download-{}",uuid::Uuid::new_v4()));fs::create_dir_all(&dir).map_err(|e|e.to_string())?;crate::update_install::protect_directory(&dir)?;
                fs::write(dir.join("owned-by-updater"),b"nogirem").map_err(|e|e.to_string())?;
                let file=dir.join(if this.portable{"portable.exe"}else{"installer.exe"});
                // Callback is throttled with an independent mutex; never hold state during I/O.
                let at=Mutex::new(std::time::Instant::now());
                download(&m,&file,&|n|{let mut last=at.lock().unwrap();if last.elapsed()>Duration::from_millis(200)||n==m.size{*last=std::time::Instant::now();this.set("downloading",(n as f64*100./m.size as f64).min(99.9),None);}},&this.cancel)?;
                this.data.lock().unwrap().file=Some(file);this.set("downloading",100.,None);std::thread::sleep(Duration::from_millis(COMPLETION_DELAY_MS));this.set("downloaded",100.,None);Ok::<_,String>(())})();
            if let Err(e)=result{this.set("error",0.,Some(e));}this.busy.store(false,Ordering::SeqCst);
        });Ok(self.state())
    }
    pub fn prepared(&self)->Result<(Manifest,Vec<u8>,PathBuf)>{
        let d=self.data.lock().unwrap();if d.state["phase"]!="downloaded"{return Err("다운로드가 완료되지 않았습니다".into());}
        let m=d.manifest.clone().ok_or("업데이트 정보 누락")?;let file=d.file.clone().ok_or("설치파일 누락")?;verify_file(&file,&m)?;Ok((m,d.envelope.clone(),file))
    }
    pub fn cancel(&self){self.cancel.store(true,Ordering::SeqCst);}
    pub fn installing(&self){self.set("installing",100.,None);}
    pub fn failed(&self,error:String){self.set("error",0.,Some(error));}
}
#[cfg(test)]
mod tests {
    use super::*;use ed25519_dalek::{Signer,SigningKey};
    fn sample()->(Manifest,Config,SigningKey){let key=SigningKey::from_bytes(&[7;32]);let cfg=Config{repository:"test/repo".into(),public_key:STANDARD.encode(key.verifying_key().as_bytes())};let m=Manifest{schema_version:1,version:"0.4.1".into(),url:"https://github.com/test/repo/releases/download/v0.4.1/nogirem-dioxus-setup-0.4.1.exe".into(),size:3,sha256:http::sha256(b"abc"),notes:String::new()};(m,cfg,key)}
    fn signed(m:&Manifest,k:&SigningKey)->Vec<u8>{let payload=serde_json::to_string(m).unwrap();serde_json::to_vec(&json!({"signature":STANDARD.encode(k.sign(payload.as_bytes()).to_bytes()),"payload":payload})).unwrap()}
    #[test] fn signatures_versions_and_addresses(){let(mut m,c,k)=sample();let good=signed(&m,&k);assert_eq!(verify_envelope(&good,&c).unwrap().version,"0.4.1");let mut portable=m.clone();portable.url="https://github.com/test/repo/releases/download/v0.4.1/nogirem-dioxus-portable-0.4.1.exe".into();assert_eq!(verify_envelope_for(&signed(&portable,&k),&c,true).unwrap().version,"0.4.1");assert!(verify_envelope_for(&good,&c,true).is_err());let mut bad:Value=serde_json::from_slice(&good).unwrap();bad["payload"]=json!(bad["payload"].as_str().unwrap().replace("0.4.1","0.9.0"));assert!(verify_envelope(&serde_json::to_vec(&bad).unwrap(),&c).is_err());m.url="https://evil.invalid/install.exe".into();assert!(verify_envelope(&signed(&m,&k),&c).is_err());assert!(newer("0.4.10","0.4.9").unwrap());assert!(!newer("0.4.0","0.4.0").unwrap());assert!(!newer("0.3.17","0.4.0").unwrap());}
    #[test] fn partial_corrupt_oversized_and_cancelled_downloads_never_survive(){let(m,_,_)=sample();for (data,cancelled) in [(b"ab".as_slice(),false),(b"abd",false),(b"abcd",false),(b"abc",true)]{let p=std::env::temp_dir().join(format!("nogirem-download-test-{}",uuid::Uuid::new_v4()));assert!(write_download(data,&p,&m,&|_|{},&AtomicBool::new(cancelled)).is_err());assert!(!p.exists());}}
    #[test] fn valid_download_is_reverified_before_install(){let(m,_,_)=sample();let p=std::env::temp_dir().join(format!("nogirem-download-test-{}",uuid::Uuid::new_v4()));write_download(b"abc".as_slice(),&p,&m,&|_|{},&AtomicBool::new(false)).unwrap();verify_file(&p,&m).unwrap();fs::write(&p,b"abd").unwrap();assert!(verify_file(&p,&m).is_err());fs::remove_file(p).unwrap();}
    #[test] fn local_blocking_download_works_without_runtime(){let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let address=listener.local_addr().unwrap();let server=std::thread::spawn(move||{let(mut stream,_)=listener.accept().unwrap();let mut request=[0u8;4096];let _=stream.read(&mut request).unwrap();stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc").unwrap();});let(m,_,_)=sample();let m=Manifest{url:format!("http://{address}/installer.exe"),..m};let p=std::env::temp_dir().join(format!("nogirem-local-download-test-{}",uuid::Uuid::new_v4()));download_with_client(&download_client(false).unwrap(),&m,&p,&|_|{},&AtomicBool::new(false)).unwrap();verify_file(&p,&m).unwrap();fs::remove_file(p).unwrap();server.join().unwrap();}
    #[test]
    #[ignore="공개 GitHub 자산을 실제로 다운로드하는 배포 전 검사"]
    fn live_signed_update_download_uses_blocking_client_without_runtime(){let(m,_)=fetch_manifest().unwrap();let p=std::env::temp_dir().join(format!("nogirem-live-download-test-{}",uuid::Uuid::new_v4()));download(&m,&p,&|_|{},&AtomicBool::new(false)).unwrap();verify_file(&p,&m).unwrap();fs::remove_file(p).unwrap();}
}
