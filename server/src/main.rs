use axum::{extract::{Form,Query,State},http::StatusCode,response::{Html,IntoResponse,Redirect},routing::{get,post},Json,Router};
use axum_extra::extract::CookieJar;
use chrono::{NaiveTime,Utc};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_postgres::{Client,NoTls};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer,trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt,util::SubscriberInitExt};
use uuid::Uuid;

const CSS:&str=include_str!("../infra/templates/voxel.css");
const W9_DB:&str="https://db.w9.nu";

#[derive(Clone)]
pub struct AppState{
    pub db:Arc<Client>,
    pub http_client:reqwest::Client,
    pub google_id:String,
    pub mail_api:String,
    pub mail_token:String,
    pub ai_key:String
}

fn layout(t:&str,b:&str,n:&str)->String{
    format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"/><meta name="viewport" content="width=device-width,initial-scale=1.0"/><title>{} — W9 Reminders</title><style>{}</style></head><body><div class="app"><nav class="nav"><a href="/" class="brand">⏰ W9 Reminders</a>{}</nav>{}</div></body></html>"#,t,CSS,n,b)
}
fn pub_layout(t:&str,b:&str)->String{layout(t,b,r#"<a href="/login">Login</a>"#)}
fn user_layout(t:&str,b:&str)->String{layout(t,b,r#"<a href="/schedules">Schedules</a><a href="/log">Log</a><a href="/logout">Logout</a>"#)}

fn set_s(j:CookieJar,t:String)->CookieJar{
    j.add(axum_extra::extract::cookie::Cookie::build(("w9_rem_session",t)).path("/").http_only(true).same_site(axum_extra::extract::cookie::SameSite::Lax).max_age(time::Duration::days(7)).finish())
}
fn clr_s(j:CookieJar)->CookieJar{j.remove(axum_extra::extract::cookie::Cookie::named("w9_rem_session"))}
fn get_s(j:&CookieJar)->Option<String>{j.get("w9_rem_session").map(|c|c.value().to_string())}

async fn verify(a:&AppState,t:&str)->Option<serde_json::Value>{
    let r=a.http_client.get(format!("{}/api/auth/me",W9_DB)).header("Authorization",format!("Bearer {}",t)).send().await.ok()?;
    if r.status().is_success(){r.json().await.ok()}else{None}
}
async fn require(j:&CookieJar,a:&AppState)->Option<serde_json::Value>{let t=get_s(j)?;verify(a,&t).await}

fn home_html()->String{
    pub_layout("W9 Reminders",r#"<div class="hero"><h1>⏰ W9 Daily Reminders</h1><p>AI-powered daily email digests from your Google Calendar</p><div class="flex mt-3" style="justify-content:center"><a href="/login" class="btn">Login with W9</a></div></div><div class="grid mt-3"><div class="card"><h3>📅 Google Calendar</h3><p class="text-sm">Connect your Google Calendar for daily event summaries.</p></div><div class="card"><h3>🤖 AI Summaries</h3><p class="text-sm">AI generates personalized daily summaries with images.</p></div><div class="card"><h3>📧 Email Delivery</h3><p class="text-sm">Beautiful HTML emails delivered via W9 Mail every morning.</p></div></div>"#)
}
fn login_html()->String{
    pub_layout("Login",r#"<div class="card" style="max-width:420px;margin:3rem auto;text-align:center"><h1>⏰ W9 Reminders</h1><p class="text-sm text-muted mb-2">Sign in with W9 DB</p><a href="https://db.w9.nu/oauth/authorize?redirect_uri=https://reminder.w9.nu/oauth/callback&response_type=code&client_id=w9-reminders" class="btn" style="width:100%">Login with W9 DB</a></div>"#)
}

fn sched_html(s:&[(String,String,String,bool,String)],m:Option<&str>)->String{
    let al=m.map(|x|format!(r#"<div class="alert alert--ok">{}</div>"#,x)).unwrap_or_default();
    let rows:String=s.iter().map(|(e,t,z,a,_)|{
        let ab=if*a{r#"<span class="badge badge--ok">Active</span>"#}else{r#"<span class="badge badge--err">Paused</span>"#};
        format!(r#"<tr><td>{}</td><td>{} ({})</td><td>{}</td></tr>"#,e,t,z,ab)
    }).collect();
    user_layout("Schedules",&format!(r#"<div class="card" style="max-width:700px;margin:2rem auto"><h1>📅 Schedules</h1>{}<form method="POST" action="/schedules"><label>Email</label><input type="email" name="user_email" required placeholder="you@w9.nu"/><label>Time (HH:MM:SS)</label><input type="text" name="send_time" value="08:00:00" required/><label>Timezone</label><input type="text" name="timezone" value="Asia/Ho_Chi_Minh"/><button type="submit" class="btn mt-1" style="width:100%">Create</button></form><h2 class="mt-3">Active</h2><table><tr><th>Email</th><th>Time</th><th>Status</th></tr>{}</table></div>"#,al,rows))
}

fn log_html(l:&[(String,String,i32,bool)])->String{
    let rows:String=l.iter().map(|(s,e,v,sent)|{
        let b=if*sent{r#"<span class="badge badge--ok">Sent</span>"#}else{r#"<span class="badge badge--err">Failed</span>"#};
        format!(r#"<tr><td class="text-xs">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,s,e,v,b)
    }).collect();
    user_layout("Log",&format!(r#"<div class="card" style="max-width:800px;margin:2rem auto"><h1>📊 Log</h1><table><tr><th>Schedule</th><th>Executed</th><th>Events</th><th>Status</th></tr>{}</table></div>"#,rows))
}

#[derive(Debug,Deserialize)]
struct SchedReq{user_email:String,send_time:String,timezone:Option<String>}

async fn home()->Html<String>{Html(home_html())}
async fn login_page()->Html<String>{Html(login_html())}

async fn oauth_cb(State(s):State<AppState>,jar:CookieJar,Query(q):Query<serde_json::Value>)->impl IntoResponse{
    let code=match q.get("code").and_then(|v|v.as_str()){Some(c)=>c.to_string(),None=>return Html(login_html()).into_response()};
    let res=match s.http_client.post(format!("{}/oauth/token",W9_DB)).form(&[("grant_type","authorization_code"),("code",&code),("redirect_uri","https://reminder.w9.nu/oauth/callback")]).send().await{Ok(r)=>r,Err(_)=>return Html(login_html()).into_response()};
    let json=match res.json::<serde_json::Value>().await{Ok(j)=>j,Err(_)=>return Html(login_html()).into_response()};
    let token=match json.get("access_token").and_then(|v|v.as_str()){Some(t)=>t.to_string(),None=>return Html(login_html()).into_response()};
    (set_s(jar,token),Redirect::to("/schedules")).into_response()
}
async fn logout(jar:CookieJar)->impl IntoResponse{(clr_s(jar),Redirect::to("/")).into_response()}

async fn sched_page(State(s):State<AppState>,jar:CookieJar)->impl IntoResponse{
    if require(&jar,&s).await.is_none(){return Redirect::to("/login").into_response();}
    let sc=match s.db.query("SELECT user_email,send_time::text,timezone,is_active,created_at::text FROM reminder_schedules ORDER BY created_at DESC",&[]).await{
        Ok(r)=>r.iter().map(|x|(x.get(0),x.get(1),x.get(2),x.get(3),x.get(4))).collect(),Err(_)=>Vec::new()};
    Html(sched_html(&sc,None)).into_response()
}

async fn sched_post(State(s):State<AppState>,jar:CookieJar,Form(f):Form<SchedReq>)->impl IntoResponse{
    if require(&jar,&s).await.is_none(){return Redirect::to("/login").into_response();}
    let _t=match NaiveTime::parse_from_str(&f.send_time,"%H:%M:%S"){Ok(t)=>t,Err(_)=>return Html(sched_html(&[],Some("Invalid time"))).into_response()};
    let id=Uuid::new_v4();
    let tz=f.timezone.as_deref().unwrap_or("Asia/Ho_Chi_Minh");
    let _=s.db.execute("INSERT INTO reminder_schedules(id,user_email,send_time,timezone,is_active,include_image)VALUES($1,$2,$3,$4,$5,$6)",&[&id,&f.user_email,&f.send_time,&tz,&true,&true]).await;
    let sc=match s.db.query("SELECT user_email,send_time::text,timezone,is_active,created_at::text FROM reminder_schedules ORDER BY created_at DESC",&[]).await{
        Ok(r)=>r.iter().map(|x|(x.get(0),x.get(1),x.get(2),x.get(3),x.get(4))).collect(),Err(_)=>Vec::new()};
    Html(sched_html(&sc,Some("Schedule created"))).into_response()
}

async fn log_page(State(s):State<AppState>,jar:CookieJar)->impl IntoResponse{
    if require(&jar,&s).await.is_none(){return Redirect::to("/login").into_response();}
    let l=match s.db.query("SELECT schedule_id::text,executed_at::text,events_count,email_sent FROM reminder_execution_log ORDER BY executed_at DESC LIMIT 50",&[]).await{
        Ok(r)=>r.iter().map(|x|(x.get(0),x.get(1),x.get(2),x.get(3))).collect(),Err(_)=>Vec::new()};
    Html(log_html(&l)).into_response()
}

async fn health(State(s):State<AppState>)->impl IntoResponse{
    match s.db.query_one("SELECT 1",&[]).await{
        Ok(_)=>(StatusCode::OK,Json(serde_json::json!({"status":"ok","service":"w9-daily-reminders","database":"connected","timestamp":Utc::now().to_rfc3339()}))),
        Err(e)=>(StatusCode::SERVICE_UNAVAILABLE,Json(serde_json::json!({"status":"error","error":e.to_string()})))
    }
}

#[tokio::main]
async fn main()->anyhow::Result<()>{
    tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_|"info".into())).with(tracing_subscriber::fmt::layer()).init();
    dotenvy::dotenv().ok();
    let port=std::env::var("PORT").unwrap_or_else(|_|"8084".into());
    let db_url=std::env::var("W9_REMINDERS_DB_URL").or_else(|_|std::env::var("DATABASE_URL")).unwrap_or_else(|_|"postgres://w9_admin:password@w9-postgres:5432/w9_reminders".into());
    let g_id=std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default();
    let mail_api=std::env::var("W9_MAIL_API_URL").unwrap_or_else(|_|"https://mail.w9.nu".into());
    let mail_tok=std::env::var("W9_MAIL_API_TOKEN").unwrap_or_default();
    let ai=std::env::var("POLLINATIONS_API_KEY").unwrap_or_default();
    let(client,conn)=tokio_postgres::connect(&db_url,NoTls).await?;
    tokio::spawn(async move{if let Err(e)=conn.await{tracing::error!("DB:{}",e);}});
    client.query_one("SELECT 1",&[]).await?;
    let state=AppState{db:Arc::new(client),http_client:reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build()?,google_id:g_id,mail_api:mail_api,mail_token:mail_tok,ai_key:ai};
    let router=Router::new()
        .route("/",get(home))
        .route("/login",get(login_page))
        .route("/oauth/callback",get(oauth_cb))
        .route("/logout",get(logout))
        .route("/schedules",get(sched_page))
        .route("/schedules",post(sched_post))
        .route("/log",get(log_page))
        .route("/api/health",get(health))
        .with_state(state)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()));
    let addr=format!("0.0.0.0:{}",port);
    let listener=TcpListener::bind(&addr).await?;
    tracing::info!("W9 Reminders on {}",addr);
    axum::serve(listener,router).await?;
    Ok(())
}
