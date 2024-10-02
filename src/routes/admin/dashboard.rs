use actix_web::HttpResponse;

pub async fn dashboard() -> HttpResponse {
    HttpResponse::Ok().finish()
}
