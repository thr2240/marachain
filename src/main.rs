extern crate walletlib;
use actix_cors::Cors;
use mara_fed_peer::peer_utils::{SwarmHandle};
use std::{time::Duration};
use actix_web::{App, HttpServer, web, http};
use actix_web::middleware::Logger;
use env_logger::Env;
use actix_rt::time;
use mara_fed_peg::peg_utils::{check_initailize_peg, tick_confirmation};
use mara_fed_wallet::wallet_utils::{node_scheduler, sync_database, send_admin_peer};
mod mara_fed_wallet;
mod mara_fed_member;
mod mara_fed_peg;
mod mara_fed_peer;
mod mara_cli;
mod mara_fed_transaction;
mod mara_fed_scanner;
use mara_fed_scanner::scanner_utils::{ listen_zmq,listen_sidechain_zmq };
use rusqlite::Connection;


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = sync_database().await;
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    let swarm_obj = SwarmHandle::new().await.unwrap();

    actix_rt::spawn(async {
        let _ = listen_sidechain_zmq();  
    });

    actix_rt::spawn(async {
        let _ = listen_zmq();  
    });

    actix_rt::spawn(async move {
        let mut total_seconds = 0;
        let conn = Connection::open(env!("DATABASE").to_owned()).unwrap(); 
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            total_seconds = total_seconds + 10;
            let _ : Result<(), rusqlite::Error>= node_scheduler(&conn);
            let _ = send_admin_peer(swarm_obj.clone(), &conn);
            if total_seconds == 60 {
                total_seconds = 0;
                check_initailize_peg(&conn);
            } else {
                tick_confirmation(&conn);
            }
        }
    });


    HttpServer::new(move || {
	let cors = Cors::default()
            .allowed_origin("http://localhost:4200")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
     	     .wrap(cors)
             .wrap(Logger::default())
             .wrap(Logger::new("%a %t %r %s %b %{Referer}i %{User-Agent}i %T"))
             .service(
                 web::scope("/wallet")
                .service(mara_fed_wallet::install)
                .service(mara_fed_wallet::deposit_address)
                .service(mara_fed_wallet::list_wallets)
             )
             .service(
                web::scope("/members")
               .service(mara_fed_member::listmembers)
            ).service(
                web::scope("/transaction")
                .service(mara_fed_transaction::sign)
                .service(mara_fed_transaction::sum_of_deposit)
                .service(mara_fed_transaction::sum_of_withdraw)
                .service(mara_fed_transaction::chart_of_deposit)
                .service(mara_fed_transaction::chart_of_withdraw)
                .service(mara_fed_transaction::address_listing)
                .service(mara_fed_transaction::history_listing)
                .service(mara_fed_transaction::fee_listing)
                .service(mara_fed_transaction::balance_listing)
            ).service(
                web::scope("/scanner")
               .service(mara_fed_scanner::listdeposits)
               .service(mara_fed_scanner::listwithdraws)
            )
    })
    .bind(("0.0.0.0", 3002))?
    .run()
    .await


}


