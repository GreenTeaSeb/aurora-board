use super::user::{get_user_boards, UserBoards};
use crate::handlers::user::{get_by_id, User};
use actix_identity::Identity;
use actix_web::{
    web::{self, Query},
    HttpResponse,
};
use sailfish::TemplateOnce;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool};

#[derive(Serialize, FromRow, Debug)]
pub struct BoardPost {
    pub id: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub poster_id: u32,
    pub board_name: Option<String>,
    pub board_icon: Option<String>,
    pub title: String,
    pub text: String,
    pub status: Option<i8>,
}

#[derive(TemplateOnce)]
#[template(path = "home.html", escape = false)]
struct HomeTemplate {
    user: anyhow::Result<User>,
    top_boards: Vec<BoardEntry>,
    user_boards: Vec<UserBoards>,
    posts: Vec<BoardPost>,
}

struct BoardEntry {
    name: String,
    member_count: u64,
    icon: String,
}

#[derive(Deserialize, Debug)]
enum Filter {
    Feed,
    All,
}

#[derive(Deserialize, Debug)]
pub struct GetPostsQuery {
    filter: Option<Filter>,
    page: Option<u32>,
    search: Option<String>,
}

async fn get_top(pool: &MySqlPool) -> Result<Vec<BoardEntry>, sqlx::Error> {
    sqlx::query_as!(
        BoardEntry,
        r#"
   SELECT boards.name, cast(count(members.board_id) as UNSIGNED) as member_count, boards.icon  from boards
left join members
on boards.id = members.board_id
group by boards.id
order by member_count desc
limit 10;"#
    )
    .fetch_all(pool)
    .await
}

pub async fn get_member_posts(
    id: u32,
    pool: &MySqlPool,
    limit: u32,
    offset: u32,
    search_param: String,
) -> Vec<BoardPost> {
    sqlx::query_as!(
        BoardPost,
        r#"
        SELECT posts.id ,posts.created_at ,posts.poster_id ,( select name from boards where id = posts.board_id) as board_name, ( select icon from boards where id = posts.board_id) as board_icon ,posts.title ,posts.`text` , likes.is_like as status FROM posts
        INNER JOIN members on members.board_id = posts.board_id 
        lEFT JOIN likes on likes.post_id = id and likes.user_id = ?
        where members.user_id  = ? and (lower(text) like ? or lower(title) like ?)
        ORDER BY posts.created_at DESC
        limit ?
        offset ?;
        "#,
        id, 
        id,
        format!("%{}%",search_param),
        format!("%{}%",search_param),
        limit,
        offset * limit
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn get_all_posts(
    pool: &MySqlPool,
    limit: u32,
    offset: u32,
    id: u32,
    search_param: String,
) -> Vec<BoardPost> {
    sqlx::query_as!(
        BoardPost,
        r#"
        SELECT posts.id ,posts.created_at ,posts.poster_id ,( select name from boards where id = posts.board_id) as board_name, ( select icon from boards where id = posts.board_id) as board_icon ,posts.title ,posts.`text`,likes.is_like as status FROM posts
        lEFT JOIN likes on likes.post_id = id and likes.user_id = ?
        WHERE (lower(text) like ? or lower(title) like ?)
        ORDER BY posts.created_at DESC
        limit ?
        offset ?;
        "#,
        id,
        format!("%{}%",search_param),
        format!("%{}%",search_param),
        limit,
        offset * limit
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn home(
    id: Identity,
    pool: web::Data<MySqlPool>,
    q: Query<GetPostsQuery>,
) -> HttpResponse {
    let id_int: u32 = id
        .identity()
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();
    let user_res = get_by_id(&id_int, pool.get_ref()).await;
    let top: Vec<BoardEntry> = get_top(pool.get_ref()).await.unwrap_or_default();
    let temp = HomeTemplate {
        user: user_res,
        top_boards: top,
        user_boards: get_user_boards(id_int, pool.get_ref()).await,
        posts: match q.filter.as_ref().unwrap_or(&Filter::All) {
            Filter::Feed => {
                get_member_posts(
                    id_int,
                    pool.get_ref(),
                    10,
                    q.page.unwrap_or_default(),
                    q.search.to_owned().unwrap_or_default(),
                )
                .await
            }
            Filter::All => {
                get_all_posts(
                    pool.get_ref(),
                    10,
                    q.page.unwrap_or_default(),
                    id_int,
                    q.search.to_owned().unwrap_or_default(),
                )
                .await
            }
        },
    };
    HttpResponse::Ok().body(temp.render_once().unwrap())
}
