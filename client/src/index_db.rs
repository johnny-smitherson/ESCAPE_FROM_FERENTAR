use dioxus::prelude::*;
use dioxus_logger::tracing::{error, info};
use indexed_db_futures::database::Database;
use indexed_db_futures::prelude::*;
use indexed_db_futures::transaction::TransactionMode;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ImageCacheRow {
    pub id: [i32; 3],
    pub img_b64: String,
}

pub type DbReesource = Resource<Result<Database, String>>;

pub fn init_db_globals() {
    let db_res = use_resource(move || async move {
        info!("indexdb open start...");
        let d = _do_init_db().await;
        info!("indexdb open done.");
        d
    });
    use_context_provider::<DbReesource>(|| db_res);
    // ensure calling component is reset on db opened
    let db_res = use_context::<DbReesource>();
    let _ = db_res.read();
}

pub async fn read_image(key: (i32, i32, i32)) -> anyhow::Result<Option<ImageCacheRow>> {
    let db_res = use_context::<DbReesource>();
    let db = match db_res.peek().as_ref() {
        None => anyhow::bail!("read_image(): db not connected yet."),
        Some(Err(e)) => anyhow::bail!("read_image(): db connection error: {:?}", e),
        Some(Ok(db)) => db.clone(),
    };
    match _do_read_image(&db, [key.0, key.1, key.2]).await {
        Ok(i) => Ok(i),
        Err(e) => anyhow::bail!(
            "read_image(): error fetching img from local storage: {:?}",
            e
        ),
    }
}
pub async fn write_image(key: (i32, i32, i32), val: &str) -> anyhow::Result<()> {
    let db_res = use_context::<DbReesource>();
    let db = match db_res.peek().as_ref() {
        None => anyhow::bail!("write_image(): db not connected yet."),
        Some(Err(e)) => anyhow::bail!("write_image(): db connection error: {:?}", e),
        Some(Ok(db)) => db.clone(),
    };
    match _do_write_image(&db, [key.0, key.1, key.2], val).await {
        Ok(_) => Ok(()),
        Err(e) => anyhow::bail!(
            "write_image(): error writing img into local storage: {:?}",
            e
        ),
    }
}

async fn _do_init_db() -> Result<Database, String> {
    info!("_do_init_db(): starting...");
    let db = Database::open("image_db4")
        .with_version(1u8)
        .with_on_upgrade_needed(|event, db| {
            match (event.old_version(), event.new_version()) {
                (0.0, Some(1.0)) => {
                    info!("_do_init_db(): creating object store 'image_store'...");
                    db.create_object_store("image_store")
                        .with_auto_increment(false)
                        .with_key_path(indexed_db_futures::KeyPath::One("id"))
                        .build()?;
                }
                _ => {
                    error!("_do_init_db(): error upgrading localdb.");
                }
            }

            Ok(())
        })
        .await;
    match db {
        Ok(db) => {
            info!("_do_init_db(): open OK.");
            Ok(db)
        }
        Err(db) => Err(format!("_do_init_db(): error opending db: {:?}", db)),
    }
}

async fn _do_write_image(
    db: &Database,
    key: [i32; 3],
    val: &str,
) -> indexed_db_futures::OpenDbResult<()> {
    // Populate some data
    let transaction = db
        .transaction("image_store")
        .with_mode(TransactionMode::Readwrite)
        .build()?;

    let store = transaction.object_store("image_store")?;

    // awaiting individual requests is optional - they still go out
    store
        .put(ImageCacheRow {
            id: key,
            img_b64: val.to_string(),
        })
        .serde()?;

    // Unlike JS, transactions ROLL BACK INSTEAD OF COMMITTING BY DEFAULT
    transaction.commit().await?;

    Ok(())
}

async fn _do_read_image(
    db: &Database,
    key: [i32; 3],
) -> indexed_db_futures::OpenDbResult<Option<ImageCacheRow>> {
    // Populate some data
    let transaction = db
        .transaction("image_store")
        .with_mode(TransactionMode::Readonly)
        .build()?;

    let store = transaction.object_store("image_store")?;

    // awaiting individual requests is optional - they still go out

    Ok(store.get(key).serde()?.await?)
}
