//! http.rs

use actix_web::HttpResponse;
// use reqwest::Response;
// use serde::Deserialize;
// use crate::error::TradeWebError;

/// 302 redirect to the relative root "/"
/// authorization: TBD
pub async fn redirect_home()->HttpResponse{
    tracing::debug!("[redirect_home]");
    // redirect to home via 302
    // https://docs.rs/actix-web/latest/actix_web/http/struct.StatusCode.html#associatedconstant.FOUND
    // https://www.rfc-editor.org/rfc/rfc7231#section-6.4.3
    HttpResponse::Found()
        .append_header(("location", "/"))
        .append_header(("Cache-Control", "no-store"))
        .finish()

}

// pub async fn parse_http_result_to_vec<'a, T: Deserialize<'a>>(http_result:Result<Response,reqwest::Error>)->Result<Vec<T>,TradeWebError>{
//     let return_val = match http_result {
//         Ok(response) => {
//             match response.text().await{
//                 Ok(response_text)=>{
//
//                     let rt2 = response_text.clone();
//
//                     match serde_json::from_str::<Vec<T>>(&rt2){
//                         Ok(results)=> {
//                             Ok(results)
//                         },
//                         Err(e)=>{
//                             tracing::debug!("[get_remote] deserialization to json vec failed: {:?}", &e);
//                             Err(TradeWebError::JsonError)
//                         }
//                     }
//                 },
//                 Err(e)=>{
//                     tracing::debug!("[get_remote] deserialization to json text failed: {:?}", &e);
//                     Err(TradeWebError::JsonError)
//                 }
//             }
//         },
//         Err(e) => {
//             tracing::debug!("[get_remote] reqwest error: {:?}", &e);
//             Err(TradeWebError::ReqwestError)
//         }
//     };
//
//     return_val
// }
