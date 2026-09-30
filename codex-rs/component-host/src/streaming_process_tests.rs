use super::*;

const STREAMING_PLUGIN: &str = r#"
import base64,json,os,pathlib,sys,time
def physical(value):
    print(json.dumps(value,separators=(',',':')),flush=True)
def message(identity,value):
    body=json.dumps(value,separators=(',',':')).encode()
    physical({'type':'message_start','id':identity,'bytes':len(body)})
    chunks=0
    for offset in range(0,len(body),192*1024):
        physical({'type':'chunk','id':identity,'index':chunks,'data':base64.b64encode(body[offset:offset+192*1024]).decode()})
        chunks+=1
    physical({'type':'end','id':identity,'chunks':chunks})
init=json.loads(sys.stdin.readline())
state=pathlib.Path(init['state_dir'])
(state/'pid').write_text(str(os.getpid()))
mode=init['config']['mode']
ready={'type':'ready','api_version':1,'component':init['component'],'session':init['session']}
if mode=='bad_ready': ready.pop('session')
physical(ready)
if mode=='stall_request': time.sleep(60)
first=sys.stdin.readline()
if not first: sys.exit(0)
(state/'request_seen').write_text('yes')
start=json.loads(first)
assert start['type']=='message_start' and start['id']==1
body=bytearray()
count=0
while True:
    frame=json.loads(sys.stdin.readline())
    assert frame['id']==1
    if frame['type']=='end':
        assert frame['chunks']==count and len(body)==start['bytes']
        break
    assert frame['type']=='chunk' and frame['index']==count
    body.extend(base64.b64decode(frame['data'],validate=True));count+=1
request=json.loads(body)
assert request['id']==1 and request['method']=='model.stream'
if mode=='large':
    message(1,{'type':'event','id':1,'event':request['params']})
    message(2,{'type':'event','id':1,'event':{'order':2}})
    message(3,{'type':'result','id':1,'result':{}})
elif mode=='partial':
    body=json.dumps({'type':'event','id':1,'event':{'drained':True}}).encode()
    physical({'type':'message_start','id':1,'bytes':len(body)})
    line=json.dumps({'type':'chunk','id':1,'index':0,'data':base64.b64encode(body).decode()})+'\n'
    sys.stdout.write(line[:12]);sys.stdout.flush()
    (state/'partial').write_text('ready')
    assert json.loads(sys.stdin.readline())=={'type':'shutdown'}
    sys.stdout.write(line[12:]);sys.stdout.flush()
    physical({'type':'end','id':1,'chunks':1})
    message(2,{'type':'result','id':1,'result':{}})
    sys.exit(0)
elif mode=='trailing':
    message(1,{'type':'result','id':1,'result':{}})
    message(2,{'type':'event','id':1,'event':'must reject'})
elif mode=='skipped': physical({'type':'message_start','id':2,'bytes':2})
elif mode=='duplicate_field': print('{"type":"message_start","id":1,"id":1,"bytes":2}',flush=True)
elif mode=='unknown_field': physical({'type':'message_start','id':1,'bytes':2,'private':'sentinel'})
elif mode=='short':
    physical({'type':'message_start','id':1,'bytes':2})
    physical({'type':'chunk','id':1,'index':0,'data':'eA=='})
    physical({'type':'end','id':1,'chunks':1})
elif mode=='partial_eof':
    physical({'type':'message_start','id':1,'bytes':2})
    sys.exit(0)
elif mode=='wrong_inner': message(1,{'type':'result','id':2,'result':{}})
assert json.loads(sys.stdin.readline())=={'type':'shutdown'}
"#;

fn streaming(mode: &str) -> Fixture {
    let mut fixture = Fixture::new(mode);
    std::fs::write(&fixture.binding.args[0], STREAMING_PLUGIN).unwrap();
    fixture.binding.spec.kind = "model_transport".to_owned();
    fixture.binding.spec.contract_version = 2;
    fixture.binding.timeout_ms = 30_000;
    fixture
}

#[tokio::test]
async fn large_request_and_events_preserve_content_and_order() {
    let fixture = streaming("large");
    let params = json!({"history":"abcdefghijklmnopqrstuvwxyz".repeat(700_000)});
    let mut stream = fixture.binding.stream("model.stream", params.clone()).await.unwrap();
    assert_eq!(stream.next().await.unwrap(), StreamFrame::Event(params));
    assert_eq!(stream.next().await.unwrap(), StreamFrame::Event(json!({"order":2})));
    assert_eq!(stream.next().await.unwrap(), StreamFrame::Done(json!({})));
    #[cfg(unix)]
    assert_reaped(fixture.pid().await).await;
}

#[tokio::test]
async fn strict_negotiation_rejects_before_request_delivery() {
    let fixture = streaming("bad_ready");
    assert!(fixture.binding.call("model.stream", json!({})).await.is_err());
    assert!(!fixture.binding.state_dir.join("request_seen").exists());
    #[cfg(unix)]
    assert_reaped(fixture.pid().await).await;
}

#[tokio::test]
async fn malformed_or_post_terminal_messages_fail_and_reap() {
    for mode in ["trailing", "skipped", "duplicate_field", "unknown_field", "short", "partial_eof", "wrong_inner"] {
        let fixture = streaming(mode);
        let error = fixture.binding.call("model.stream", json!({})).await.unwrap_err();
        assert!(!format!("{error:#}").contains("sentinel"));
        #[cfg(unix)]
        assert_reaped(fixture.pid().await).await;
    }
}

#[tokio::test]
async fn graceful_control_preserves_a_partial_physical_frame() {
    let fixture = streaming("partial");
    let mut stream = fixture.binding.stream("model.stream", json!({})).await.unwrap();
    timeout(Duration::from_secs(3), async {
        while !fixture.binding.state_dir.join("partial").exists() { tokio::task::yield_now().await; }
    }).await.unwrap();
    stream.shutdown_handle().request(Duration::from_secs(2));
    assert_eq!(stream.next().await.unwrap(), StreamFrame::Event(json!({"drained":true})));
    assert_eq!(stream.next().await.unwrap(), StreamFrame::Done(json!({})));
}

#[tokio::test]
async fn forced_partial_request_joins_spool_work_and_reaps() {
    let fixture = streaming("stall_request");
    let startup = fixture.binding.start_stream("model.stream", json!({"history":"x".repeat(17 * 1024 * 1024)}));
    let pid = fixture.pid().await;
    startup.shutdown_handle().force_and_reap().await.unwrap();
    assert!(startup.ready().await.is_err());
    #[cfg(unix)]
    assert_reaped(pid).await;
}
