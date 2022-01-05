use sqlx::{FromRow, MySqlPool, types::chrono};
use serde::{Serialize, Deserialize};
use anyhow::{Result,anyhow};
use actix_web::{self, Responder, web, HttpResponse, HttpRequest};
use argonautica::{Hasher, Verifier};
use std::env;

#[derive(Serialize, FromRow, Debug)]
pub struct User{
    pub id: u32,
    pub username: String,
    pub email: String,
    pub password: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

#[derive(Serialize)]
pub struct Error{
    text: String,
}

#[derive(Deserialize)]
pub struct SignupData {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginData {
    username: String,
    password: String,
}


enum errors {
    UsernameTaken
}

async fn new_user(form: SignupData, pool: &MySqlPool) -> Result<User>{
    let users = sqlx::query_as!(User,
    "
SELECT * FROM users
WHERE username = ?
    "
    ,form.username ).fetch_all(pool).await?;

    if users.len() == 0{
	let key = env::var("KEY").expect("SECRET KEY not set");
	let pass = match Hasher::default().with_password(form.password).with_secret_key(key).configure_hash_len(32).hash(){	
            Ok(pass)  => pass,
            Err(e) => return Err(anyhow!("Error with hash {}",e)),
	};
	
	let new =sqlx::query!(
	r#"
	    INSERT INTO users (username, email, password)
	    VALUES (? , ? , ?)
	"#, form.username, form.email, pass).execute(pool).await?;

	let user = sqlx::query_as!(User,
	r#"
	    SELECT * FROM users
	    WHERE id = ? 
	"#
	,new.last_insert_id() ).fetch_one(pool).await?;

	Ok(user)
    }else{
	return Err(anyhow!("Username taken"));
    }
}

async fn login_user(form: LoginData, pool: &MySqlPool) -> Result<User>{
    let user = match sqlx::query_as!(User,
    r#"
	SELECT * FROM users
	WHERE username = ? 
    "#,form.username ).fetch_one(pool).await{
	Ok(u) => u,
	Err(_) =>  return Err(anyhow!("Username doesn't exist")),
    };
    
    let key = env::var("KEY").expect("SECRET KEY not set");
    let hash =String::from_utf8_lossy(&user.password);
    let pass = match Verifier::default().with_hash(hash).with_password(&form.password).with_secret_key(key).verify(){
        Ok(pass)  => pass,
        Err(e) => return Err(anyhow!("Error with hash {}", e)),
	};
    if pass {
	Ok(user)
    }else{
	return Err(anyhow!("Wrong password"))
    }
}



pub async fn signup(form: web::Form<SignupData>, db_pool: web::Data<MySqlPool>) -> impl Responder {
    match new_user(form.into_inner(), db_pool.as_ref()).await {
	Ok(t) => format!("{:?}",t),
	Err(e) => format!("{}",e)
    }
}

pub async fn login(form: web::Form<LoginData>, db_pool: web::Data<MySqlPool>) -> impl Responder {
   match login_user(form.into_inner(), db_pool.as_ref()).await {
       Ok(t)  => format!("{:?}",t),
       Err(e) => format!("{}",e) 
   }
}

pub async fn get(req: HttpRequest, db_pool: web::Data<MySqlPool>) -> impl Responder {
    let params = req.match_info().get("id").unwrap_or("World!");
    format!("{}",params )
}



