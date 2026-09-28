import base64,json,os,queue,shutil,subprocess,threading,time
from pathlib import Path
repo=Path(__file__).resolve().parent.parent
import tempfile
work=Path(tempfile.mkdtemp(prefix='nogirem-rust-controller-'))
directory=work/f'controller-integration-{int(time.time())}'
directory.mkdir()
config=json.loads((repo/'config.json').read_text(encoding='utf-8-sig'))
config.update(gameExecutable=str(directory/'NoSuchGame.exe'),gameExecutableName='NoSuchGame.exe',gameDirectoryNames=['NoSuchGame'],gameDirectoryName='NoSuchGame',pollIntervalMs=50)
config['memoryCleaner']['pollIntervalMs']=50
(directory/'config.json').write_text(json.dumps(config),encoding='utf-8')
for relative in ['assets/주변캐릭터간소화프레임제한해제.muo','native/recorder-helper/tools-bin/recorder-helper.exe']:
    target=directory/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(repo/relative,target)
user=directory/'user';videos=directory/'videos';documents=directory/'documents'
env=dict(os.environ,NOGIREM_ROOT=str(directory),NOGIREM_USER_DATA=str(user),NOGIREM_PACKAGED='0',NOGIREM_KNOWN_FOLDERS=json.dumps(dict(videos=str(videos),documents=str(documents),downloads=str(directory))),NOGIREM_DISPLAYS=json.dumps([dict(bounds=dict(x=0,y=0,width=1920,height=1080),workArea=dict(x=0,y=0,width=1920,height=1040))]))
process=subprocess.Popen([os.environ.get('NOGIREM_TEST_EXE', str(repo/'desktop/target/debug/nogirem-desktop.exe')),'--native-service'],cwd=directory,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',creationflags=subprocess.CREATE_NO_WINDOW)
lock=threading.Lock();pending={};opened=threading.Event();sequence=0;outgoing=[];errors=[]
def send(value):
    with lock:process.stdin.write(json.dumps(value,ensure_ascii=False)+'\n');process.stdin.flush()
def reader():
    for line in process.stdout:
        value=json.loads(line)
        if value.get('type')=='response':
            reply=pending.get(value['id'])
            if reply:reply.put(value)
        elif value.get('type')=='request':
            method=value['method'];p=value.get('params',{});outgoing.append(method)
            result=None
            if method=='window.open':
                if p['windowId']==1:opened.set()
                if str(p.get('file','')).endswith('dxvk-manager.html'):
                    send(dict(type='event',name='send',params=dict(windowId=p['windowId'],channel='dxvk:content-ready',args=[])))
            elif method.startswith('dialog.'):
                result=dict(canceled=True,response=0)
            send(dict(type='response',id=value['id'],result=result))
def stderr():
    for line in process.stderr:errors.append(line.rstrip())
threading.Thread(target=reader,daemon=True).start();threading.Thread(target=stderr,daemon=True).start()
def request(method,p,timeout=90):
    global sequence
    sequence+=1;identifier=sequence;reply=queue.Queue();pending[identifier]=reply;send(dict(type='request',id=identifier,method=method,params=p))
    response=reply.get(timeout=timeout);pending.pop(identifier,None)
    if response.get('error'):raise RuntimeError(response['error'])
    return response.get('result')
def invoke(channel,*args,window=1):return request('invoke',dict(windowId=window,channel=channel,args=list(args)))
results=[]
def check(name,condition,detail=None):
    results.append(dict(case=name,passed=bool(condition),detail=detail))
    if not condition:raise AssertionError(name+': '+str(detail))
try:
    assert opened.wait(10)
    contract=request('contract',{})
    check('startup-log-written','Rust backend' in (user/'logs/startup.log').read_text(encoding='utf-8'))
    check('complete-rust-ipc-contract',len(contract['channels'])==100,len(contract['channels']))
    launch=invoke('application:get-launch-context')
    (directory/'launch.json').write_text(json.dumps(launch,ensure_ascii=False,indent=2),encoding='utf-8')
    for key in ['affinity','memory']:
        result=launch['optimizationStatus'][key]
        check(f'{key}-production-worker',result.get('ok') and result['data']['running'] and not result['data'].get('gameActive'),result)
    check('graphics-read-only',launch['optimizationStatus']['graphics'].get('ok'),launch['optimizationStatus']['graphics'])
    check('network-read-only',launch['optimizationStatus']['network'].get('ok'),launch['optimizationStatus']['network'])
    check('blackbox-disabled-does-not-start',launch['blackboxSetting']['running'] is False,launch['blackboxSetting'])
    check('simplification-installed-in-isolated-documents',(documents/'마비노기/설정/목록/주변캐릭터간소화프레임제한해제.muo').is_file())
    check('music-setting-write',invoke('application:set-startup-music-setting',True)==dict(muted=True))
    check('music-setting-persisted',json.loads((user/'startup-music.json').read_text())['muted'])
    check('input-setting-without-helper',invoke('application:set-input-guard-setting',dict(enabled=False,cursorScalePercent='100',cursorWheelModifier='disabled'))['cursorScalePercent']==100)
    invoke('application:record-creator-prompt-display');invoke('application:record-creator-prompt-display')
    check('creator-counter-once-per-run',json.loads((user/'creator/prompt.json').read_text())['displayCount']==1)
    try:invoke('application:set-startup-music-setting',True,window=999)
    except RuntimeError:check('reject-unknown-window',True)
    else:check('reject-unknown-window',False)
    clips=videos/'마비노기 렘 블랙박스/Clips';clips.mkdir(parents=True,exist_ok=True);(clips/'test.mp4').write_bytes(bytes(range(100)))
    media=request('protocol',dict(url='http://nogirem-blackbox.clips/test.mp4',method='GET',headers={'range':'bytes=10-19'}))
    check('native-media-range',media['status']==206 and base64.b64decode(media['body'])==bytes(range(10,20)),media['headers'])
    try:request('protocol',dict(url='http://nogirem-blackbox.clips/..%5Coutside.mp4',method='GET',headers={}))
    except RuntimeError:check('media-traversal-rejected',True)
    else:check('media-traversal-rejected',False)
    check('exit-keep',invoke('application:confirm-close','keep')['closing'])
    for kind in ['memory','affinity']:
        state=json.loads((user/kind/'status.json').read_text())
        check(f'{kind}-stopped',not state['running'],state.get('exitAction'))
finally:
    try:process.stdin.close()
    except Exception:pass
    try:process.wait(timeout=20)
    except subprocess.TimeoutExpired:process.kill();process.wait()
    report=dict(passed=len(results)==18 and all(r['passed'] for r in results),cases=results,stderr=errors,root=str(directory),exitCode=process.returncode)
    (work/'rust-controller-integration.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
    print(json.dumps(dict(passed=report['passed'],cases=len(results),root=str(directory),errors=errors),ensure_ascii=False))
