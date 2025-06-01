use anyhow::Result;
use pocketbase_client::admin::Admin;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // admin authentication
    let authenticated_admin_client = Admin::new("http://localhost:8090")
        .auth_with_password("", "")
        .await?;

    // collections list + Filter
    let collections = authenticated_admin_client
        .collections()
        .list()
        .page(1)
        .filter("name = 'employees'".to_string())
        .per_page(100)
        .call()
        .await?;

    dbg!(collections);

    // view collection
    let user_collection = authenticated_admin_client
        .collections()
        .view("users")
        .call()
        .await?;

    dbg!(user_collection);

    Ok(())
}
