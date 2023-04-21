use actix_web::{web::Json, Responder, Result, HttpRequest};
use rusqlite::Connection;

use super::{member_model::MemberListModel, member_utils::get_members};

pub fn listmembers() -> Result<impl Responder> {
    let conn: Connection = Connection::open(env!("DATABASE").to_owned()).unwrap();
    let obj = MemberListModel {
        data: get_members(&conn, false).unwrap(),
    };
    Ok(Json(obj))
}
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test, App};
    use rusqlite::{Connection, Result as SqlResult};
    use tempfile::NamedTempFile;

    fn setup_test_db() -> SqlResult<(Connection, NamedTempFile)> {
        let temp_db = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_db.path()).unwrap();

        conn.execute_batch(
            "
            BEGIN;
            CREATE TABLE IF NOT EXISTS federation_member (
                identity TEXT PRIMARY KEY,
                host TEXT NOT NULL,
                port TEXT NOT NULL,
                peerId TEXT NOT NULL,
                user_pub TEXT NOT NULL,
                master_pub TEXT NOT NULL,
                status TEXT NOT NULL
            );
            INSERT INTO federation_member (identity, host, port, peerId, user_pub, master_pub, status) VALUES (1, 'localhost', '8080', 'peer1', 'user_pub1', 'master_pub1', 'active');
            INSERT INTO federation_member (identity, host, port, peerId, user_pub, master_pub, status) VALUES (2, 'localhost', '8081', 'peer2', 'user_pub2', 'master_pub2', 'active');
            COMMIT;",
        )?;

        Ok((conn, temp_db))
    }

    async fn listmembers_wrapper(_req: HttpRequest) -> Result<impl Responder, actix_web::Error> {
        listmembers()
    }
    
    #[actix_rt::test]
    async fn test_listmembers() {
        // Set up the test environment
        let (conn, temp_db) = setup_test_db().unwrap();

        std::env::set_var("DATABASE", temp_db.path().to_str().unwrap());

        // Create an Actix Web test server with listmembers route
        let mut app = test::init_service(App::new().route("/listmembers", actix_web::web::get().to(listmembers_wrapper))).await;


        // Call the listmembers function through the test server
        let req = test::TestRequest::get().uri("/listmembers").to_request();
        let resp = test::call_service(&mut app, req).await;

        // Assert that the response has a status of 200 OK
        assert_eq!(resp.status(), StatusCode::OK);

        // Deserialize JSON response into a MemberListModel
        let member_list_model: MemberListModel = test::read_body_json(resp).await;

        // Assert that the result is as expected
        let members_data = &member_list_model.data;
        assert_eq!(members_data.len(), 2);
        assert_eq!(members_data[0].identity, "1");
        assert_eq!(members_data[0].host, "localhost");
        assert_eq!(members_data[0].port, "8080");
        assert_eq!(members_data[0].peer_id, "peer1");
        assert_eq!(members_data[0].user_pub, "user_pub1");
        assert_eq!(members_data[0].master_pub, "master_pub1");
        assert_eq!(members_data[0].status, "active");
        assert_eq!(members_data[1].identity, "2");
        assert_eq!(members_data[1].host, "localhost");
        assert_eq!(members_data[1].port, "8081");
        assert_eq!(members_data[1].peer_id, "peer2");
        assert_eq!(members_data[1].user_pub, "user_pub2");
        assert_eq!(members_data[1].master_pub, "master_pub2");
        assert_eq!(members_data[1].status, "inactive");
    }
}