//! Public generated media is fetched without provider credentials. Retrying a
//! bounded GET cannot create another inference request or another charge.
use std::error::Error as _;
use std::time::Duration;

const LIMIT:usize=32*1024*1024;
struct Failure{message:String,retryable:bool}
fn failure(message:String,retryable:bool)->Failure{Failure{message,retryable}}

fn validate(raw:&str,controlled:Option<&str>)->Result<reqwest::Url,Failure>{
 let url=reqwest::Url::parse(raw).map_err(|_|failure("Неверный URL результата.".into(),false))?;
 let allowed_test=controlled.and_then(|v|reqwest::Url::parse(v).ok()).is_some_and(|base|base.scheme()=="http"&&base.host_str()==Some("127.0.0.1")&&base.port().is_some()&&base.origin()==url.origin());
 let host=url.host_str().unwrap_or("");
 if !url.username().is_empty()||url.password().is_some()||url.fragment().is_some()||!allowed_test&&(url.scheme()!="https"||url.port_or_known_default()!=Some(443)||!(host=="fal.media"||host.ends_with(".fal.media")||host=="storage.googleapis.com"||host=="cdn3.pixelcut.app")){
  return Err(failure("URL результата не принадлежит доверенному хранилищу; ключ ему не отправлялся.".into(),false));
 }
 Ok(url)
}

fn transport(error:&reqwest::Error,host:&str)->Failure{
 // Do not expose signed paths/query strings or raw reqwest errors in UI/logs.
 let mut causes=String::new();let mut source=error.source();
 while let Some(e)=source{causes.push_str(&e.to_string().to_lowercase());source=e.source();}
 let certificate=causes.contains("certificate")||causes.contains("unknownissuer")||causes.contains("unknown issuer");
 let dns=causes.contains("dns")||causes.contains("resolve")||causes.contains("lookup address");
 let detail=if error.is_timeout(){"тайм-аут"}else if certificate{"TLS: сертификат сервера не подтверждён"}else if dns{"ошибка DNS"}else if error.is_connect(){"не удалось установить соединение"}else if error.is_body(){"соединение прервалось при загрузке"}else{"ошибка сетевого соединения"};
 failure(format!("Не удалось скачать изображение с {host}: {detail}. Квитанция сохранена; «Продолжить задачу» повторит получение результата без новой генерации."),!certificate)
}

async fn once(client:&reqwest::Client,raw:&str,controlled:Option<&str>)->Result<Vec<u8>,Failure>{
 let mut url=validate(raw,controlled)?;
 for redirects in 0..=3{
  let host=url.host_str().unwrap_or("").to_string();
  let mut response=client.get(url.clone()).send().await.map_err(|e|transport(&e,&host))?;
  if response.status().is_redirection(){
   if redirects==3{return Err(failure(format!("Хранилище {host} вернуло слишком много перенаправлений."),false));}
   let location=response.headers().get(reqwest::header::LOCATION).and_then(|v|v.to_str().ok()).ok_or_else(||failure(format!("Хранилище {host} не указало адрес перенаправления."),false))?;
   let next=url.join(location).map_err(|_|failure("Неверный адрес перенаправления результата.".into(),false))?;
   url=validate(next.as_str(),controlled)?;continue;
  }
  let code=response.status();
  if !code.is_success(){return Err(failure(format!("Хранилище {host} не отдало изображение: HTTP {}. Оплаченный запрос сохранён; повторная генерация не выполнялась.",code.as_u16()),matches!(code.as_u16(),408|429|500|502|503|504)));}
  if response.content_length().is_some_and(|n|n>LIMIT as u64){return Err(failure("Результат превышает 32 МиБ; квитанция сохранена.".into(),false));}
  let mut bytes=Vec::new();
  while let Some(chunk)=response.chunk().await.map_err(|e|transport(&e,&host))?{
   if chunk.len()>LIMIT-bytes.len(){return Err(failure("Результат превышает 32 МиБ; квитанция сохранена.".into(),false));}bytes.extend_from_slice(&chunk);
  }
  if bytes.is_empty(){return Err(failure(format!("Хранилище {host} вернуло пустой файл."),true));}
  return Ok(bytes);
 }
 unreachable!()
}

pub(super) async fn download(client:&reqwest::Client,raw:&str,controlled:Option<&str>)->Result<Vec<u8>,String>{
 for attempt in 0..3{
  match once(client,raw,controlled).await{
   Ok(bytes)=>return Ok(bytes),
   Err(e) if e.retryable&&attempt<2=>tokio::time::sleep(Duration::from_millis(250*(attempt+1))).await,
   Err(e)=>return Err(e.message),
  }
 }
 unreachable!()
}

#[cfg(test)]mod tests{
 use super::*;
 use std::{io::{Read,Write},net::TcpListener,sync::{Arc,Mutex},thread,time::Instant};
 fn serve(responses:Vec<String>)->(String,Arc<Mutex<Vec<String>>>,thread::JoinHandle<()>){
  let listener=TcpListener::bind("127.0.0.1:0").unwrap();let base=format!("http://{}",listener.local_addr().unwrap());listener.set_nonblocking(true).unwrap();let requests=Arc::new(Mutex::new(Vec::new()));let log=requests.clone();
  let handle=thread::spawn(move||{let deadline=Instant::now()+Duration::from_secs(8);for response in responses{loop{assert!(Instant::now()<deadline,"missing expected GET");match listener.accept(){Ok((mut socket,_))=>{socket.set_nonblocking(false).unwrap();socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let mut raw=Vec::new();let mut buffer=[0u8;1024];while !raw.windows(4).any(|v|v==b"\r\n\r\n"){let n=socket.read(&mut buffer).unwrap();if n==0{break;}raw.extend_from_slice(&buffer[..n]);assert!(raw.len()<8192);}log.lock().unwrap().push(String::from_utf8(raw).unwrap());if response!="RESET"{socket.write_all(response.as_bytes()).unwrap();}break;},Err(e)if e.kind()==std::io::ErrorKind::WouldBlock=>thread::sleep(Duration::from_millis(5)),Err(e)=>panic!("{e}")}}}});(base,requests,handle)
 }
 fn client()->reqwest::Client{reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(2)).build().unwrap()}
 #[test]fn result_urls_keep_signed_queries_and_reject_untrusted_redirect_targets(){
  assert!(validate("https://v3b.fal.media/x.png?signature=private",None).is_ok());
  for url in ["http://fal.media/x","https://fal.media.attacker.test/x","https://user:secret@fal.media/x","https://127.0.0.1/x","https://fal.media:444/x"]{assert!(validate(url,None).is_err());}
  assert!(validate("http://127.0.0.1:2222/x",Some("http://127.0.0.1:2223")).is_err());
 }
 #[tokio::test]async fn transient_http_retries_only_get_then_returns_bytes(){
  let(base,log,server)=serve(vec!["HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\npng".into()]);
  assert_eq!(download(&client(),&format!("{base}/file?signature=private"),Some(&base)).await.unwrap(),b"png");server.join().unwrap();let requests=log.lock().unwrap();assert_eq!(requests.len(),2);assert!(requests.iter().all(|r|r.starts_with("GET ")&&!r.to_lowercase().contains("authorization:")));
 }
 #[tokio::test]async fn reset_retries_but_http_403_is_reported_without_url_query(){
  let(base,log,server)=serve(vec!["RESET".into(),"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()]);
  let error=download(&client(),&format!("{base}/file?signature=private"),Some(&base)).await.unwrap_err();server.join().unwrap();assert!(error.contains("HTTP 403"));assert!(!error.contains("private")&&!error.contains("signature"));assert_eq!(log.lock().unwrap().len(),2);
 }
 #[tokio::test]async fn redirect_is_validated_and_oversize_fails_before_reading(){
  let(base,log,server)=serve(vec!["HTTP/1.1 302 Found\r\nLocation: https://untrusted.invalid/file?secret=private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()]);
  let error=download(&client(),&base,Some(&base)).await.unwrap_err();server.join().unwrap();assert!(!error.contains("private"));assert_eq!(log.lock().unwrap().len(),1);
  let(base,log,server)=serve(vec![format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",LIMIT+1)]);assert!(download(&client(),&base,Some(&base)).await.unwrap_err().contains("32"));server.join().unwrap();assert_eq!(log.lock().unwrap().len(),1);
 }
}
