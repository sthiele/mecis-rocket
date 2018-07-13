#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate dotenv;
extern crate mysql;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use dotenv::dotenv;
use mysql as my;
use rocket::response::NamedFile;
use rocket::response::Stream;
use rocket::State;
use rocket_contrib::Json;
use std::cmp::Ordering;
use std::env;
use std::io::Cursor;

const MAX_IS_SIZE: u32 = 20;
const ROWSPERSITE: u32 = 50;
const ROWSPERFILE: u32 = 10000;

pub fn establish_connection() -> my::Pool {
    dotenv().ok();
    let pool = my::Pool::new(env::var("DATABASE_URL").expect("DATABASE_URL must be set")).unwrap();
    pool
}

#[get("/mecis_logo")]
fn mecis_logo() -> NamedFile {
    NamedFile::open("frontend/mecis-logo.png").unwrap()
}

#[derive(Serialize, Clone)]
struct MecisInfo {
    organisms: Vec<String>,
    models: Vec<String>,
    inreacs: Vec<String>,
    exreacs: Vec<String>,
    reactions: Vec<String>,
    mbys: Vec<f64>,
    mpys: Vec<f64>,
    scens: Vec<u32>,
}

#[get("/")]
fn mecis() -> NamedFile {
    NamedFile::open("frontend/mecis.html").unwrap()
}

#[get("/mecisinfo")]
fn mecis_info(state: State<MState>) -> Json<MecisInfo> {
    Json(state.info.clone())
}

fn create_reaction_mapping(conn: &my::Pool) -> Vec<KnockOut> {
    let mut mecisids = vec![];
    let sql = format!("SELECT mecisid FROM reactions");
    let mut stmt = conn.prepare(sql).unwrap();
    for row in stmt.execute(()).unwrap() {
        let mecisid = my::from_row::<u32>(row.unwrap());
        mecisids.push(mecisid);
    }

    let mut mapping = vec![];
    for mecisid in mecisids {
        let mut name = "".to_string();
        let sql = format!("SELECT name FROM reactions WHERE mecisid={}", mecisid);
        let mut stmt = conn.prepare(sql).unwrap();
        for row in stmt.execute(()).unwrap() {
            name = my::from_row::<String>(row.unwrap());
        }

        let mut keggid = None;
        let sql = format!("SELECT keggid FROM reactions WHERE mecisid={}", mecisid);
        let mut stmt = conn.prepare(sql).unwrap();
        for row in stmt.execute(()).unwrap() {
            if let Ok(id) = my::from_row_opt::<String>(row.unwrap()) {
                keggid = Some(id);
            } else {
                keggid = None;
            }
        }
        if keggid.is_some() {
            mapping.push(KnockOut {
                name: name,
                link: Some(format!(
                    "https://www.genome.jp/dbget-bin/www_bget?{}",
                    keggid.unwrap()
                )),
            });
        } else {
            let mut biggid = None;
            let sql = format!("SELECT biggid FROM reactions WHERE mecisid={}", mecisid);
            let mut stmt = conn.prepare(sql).unwrap();
            for row in stmt.execute(()).unwrap() {
                if let Ok(id) = my::from_row_opt::<String>(row.unwrap()) {
                    biggid = Some(id);
                } else {
                    biggid = None;
                }
            }
            if biggid.is_some() {
                mapping.push(KnockOut {
                    name: name,
                    link: Some(format!(
                        "http://bigg.ucsd.edu/universal/reactions/{} ",
                        biggid.unwrap()
                    )),
                });
            } else {
                mapping.push(KnockOut {
                    name: name,
                    link: None,
                });
            }
        }
    }
    mapping
}

fn name2id(conn: &my::Pool, name: &str) -> Option<u32> {
    let sql = format!("SELECT mecisid FROM reactions WHERE name='{}'", name);
    let mut stmt = conn.prepare(sql).unwrap();
    let mut res = None;
    for row in stmt.execute(()).unwrap() {
        res = Some(my::from_row::<u32>(row.unwrap()));
    }
    res
}

// compare f64 panic on NaN
fn mcmp(one: &f64, other: &f64) -> Ordering {
    if one.is_nan() {
        panic!("Unexpected NaN");
    }
    if other.is_nan() {
        panic!("Unexpected NaN");
    }
    one.partial_cmp(other).unwrap()
}

#[derive(FromForm, Clone)]
struct Query {
    organism: String,
    model: String,
    inreac: String,
    exreac: String,
    mby: f64,
    mpy: f64,
    scen: u32,
    mustin: String,
    forbidden: String,
    col_offset: u32,
}

fn countcis(conn: &my::Pool, q: Query) -> u32 {
    let mut sql = create_query(&conn, &q);
    sql = format!("SELECT  COUNT(*) FROM (SELECT DISTINCT organism,model,inreac,exreac,mby,mpy,scen, s FROM ({}) AS TX) AS TY", sql);
    println!("SQL: {}", sql);
    let mut stmt = conn.prepare(sql).unwrap();

    for row in stmt.execute(()).unwrap() {
        return my::from_row(row.unwrap());
    }
    return 0;
}

#[derive(Serialize, PartialOrd, PartialEq, Clone, Debug)]
struct ResponseRoW {
    organism: String,
    model: String,
    inreac: String,
    exreac: String,
    mby: f64,
    mpy: f64,
    scen: u32,
    mis: Vec<KnockOut>,
}
#[derive(Serialize, PartialOrd, PartialEq, Clone, Debug)]
struct KnockOut {
    name: String,
    link: Option<String>,
}

#[derive(Serialize, Debug)]
struct QueryResponse {
    col_offset: u32,
    max_mis: u32,
    rows: Vec<ResponseRoW>,
}

#[get("/getcis?<q>")]
fn getcis(conn: State<my::Pool>, st: State<MState>, q: Query) -> Json<QueryResponse> {
    let mapping = &st.mapping;
    let max_mis;
    if q.col_offset == 0 {
        max_mis = countcis(&conn, q.clone());
        if max_mis == 0 {
            let view = QueryResponse {
                col_offset: 0,
                max_mis: 0,
                rows: vec![],
            };
            return Json(view);
        }
    } else {
        max_mis = 0;
    }

    let mut sql = create_query(&conn, &q);
    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );
    let limit = MAX_IS_SIZE * ROWSPERSITE;
    sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, q.col_offset));
    println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let mut rows = vec![];
    let mut mis = vec![];
    let mut old_key = ResponseRoW {
        organism: "".to_string(),
        model: "".to_string(),
        inreac: "".to_string(),
        exreac: "".to_string(),
        mby: 0.0,
        mpy: 0.0,
        scen: 0,
        mis: vec![],
    };
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;
    let mut col_counter = 0;
    let tmp = stmt.execute(()).unwrap();
    for row in tmp {
        if counter > ROWSPERSITE {
            break;
        }
        col_counter = col_counter + 1;
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, usize)>(
                row.unwrap(),
            );
        let mut key = ResponseRoW {
            organism: organism,
            model: model,
            inreac: inreac,
            exreac: exreac,
            mby: mby,
            mpy: mpy,
            scen: scen,
            mis: vec![],
        };

        if old_key != key || old_set_id != s {
            if counter == ROWSPERSITE {
                col_counter = col_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = key;
                old_set_id = s;
                let ko = &mapping[r];
                mis.push(ko.clone());
            } else {
                old_key.mis = mis;
                rows.push(old_key);
                old_key = key;
                old_set_id = s;
                mis = vec![];
                let ko = &mapping[r];
                mis.push(ko.clone());
            }
        } else {
            let ko = &mapping[r];
            mis.push(ko.clone());
        }
    }

    old_key.mis = mis;
    rows.push(old_key);

    let view = QueryResponse {
        col_offset: q.col_offset + col_counter,
        max_mis: max_mis,
        rows: rows,
    };

    //     println!("view: {:?}",view);
    Json(view)
}

#[get("/getcsv?<q>")]
fn getcsv(conn: State<my::Pool>, st: State<MState>, q: Query) -> Stream<Cursor<String>> {
    let mapping = &st.mapping;
    let max_mis = countcis(&conn, q.clone());

    let mut stream = "".to_string();

    if max_mis == 0 {
        return Stream::from(Cursor::new(stream));
    }

    let mut sql = create_query(&conn, &q);
    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );
    let limit = MAX_IS_SIZE * ROWSPERFILE;
    sql.push_str(&format!(" LIMIT {}", limit));
    println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;
    let tmp = stmt.execute(()).unwrap();
    for row in tmp {
        if counter > ROWSPERFILE {
            break;
        }
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, usize)>(
                row.unwrap(),
            );
        let key = format!(
            "{},{},{},{},{},{},{}",
            organism, model, inreac, exreac, mby, mpy, scen
        );

        if old_key != key || old_set_id != s {
            if counter == ROWSPERFILE {
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = key;
                old_set_id = s;
                let ko = &mapping[r];
                mis.push_str(&format!("{} ", ko.name));
            } else {
                stream.push_str(&format!("{},{}\n", old_key, &mis));
                old_key = key;
                old_set_id = s;
                mis = "".to_string();
                let ko = &mapping[r];
                mis.push_str(&format!("{} ", ko.name));
            }
        } else {
            let ko = &mapping[r];
            mis.push_str(&format!("{} ", ko.name));
        }
    }
    stream.push_str(&format!("{},{}\n", old_key, &mis));
    Stream::from(Cursor::new(stream))
}

fn create_query(conn: &my::Pool, q: &Query) -> String {
    let mut sql = "SELECT * FROM mis WHERE 1".to_string();
    if q.organism != "None" {
        sql.push_str(" AND organism='");
        sql.push_str(&q.organism);
        sql.push('\'');
    }
    if q.model != "None" {
        sql.push_str(" AND model='");
        sql.push_str(&q.model);
        sql.push('\'');
    }
    if q.inreac != "None" {
        sql.push_str(" AND inreac='");
        sql.push_str(&q.inreac);
        sql.push('\'');
    }
    if q.exreac != "None" {
        sql.push_str(" AND exreac='");
        sql.push_str(&q.exreac);
        sql.push('\'');
    }
    if !q.mby.is_nan() {
        sql.push_str(" AND mby='");
        sql.push_str(&format!("{}", q.mby));
        sql.push('\'');
    }
    if !q.mpy.is_nan() {
        sql.push_str(" AND mpy='");
        sql.push_str(&format!("{}", q.mpy));
        sql.push('\'');
    }
    sql.push_str(" AND scen='");
    sql.push_str(&format!("{}", q.scen));
    sql.push('\'');

    let mut outer_sql = format!("SELECT * FROM ({}) AS T0 WHERE 1", sql);

    let mut mustin = q.mustin.split_whitespace();
    let mut forbidden = q.forbidden.split_whitespace();
    let mut counter = 1;

    while let Some(r) = mustin.next() {
        if let Some(mecisid) = name2id(conn, r) {
            outer_sql = format!(
            "{} AND EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, mecisid);
            counter = counter + 1;
        } else {
            outer_sql = format!("{} AND 1=0", outer_sql);
        }
    }

    while let Some(r) = forbidden.next() {
        if let Some(mecisid) = name2id(conn, r) {
            outer_sql = format!(
            "{} AND NOT EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, mecisid);
            counter = counter + 1;
        }
    }

    outer_sql
}

#[error(404)]
fn not_found(req: &rocket::request::Request) -> String {
    format!("Not found.\n Request {}", req)
}

struct MState {
    info: MecisInfo,
    mapping: Vec<KnockOut>,
}

fn rocket() -> rocket::Rocket {
    dotenv().ok();
    let pool = my::Pool::new(env::var("DATABASE_URL").expect("DATABASE_URL must be set")).unwrap();
    let info = info(&pool);
    let mapping = create_reaction_mapping(&pool);

    rocket::ignite()
        .manage(pool)
        .manage(MState {
            info: info,
            mapping: mapping,
        })
        .mount("/", routes![mecis])
        .mount("/", routes![mecis_info])
        .mount("/", routes![getcis])
        .mount("/", routes![getcsv])
        .mount("/", routes![mecis_logo])
        .catch(errors![not_found])
}

fn main() {
    rocket().launch();
}

fn info(conn: &my::Pool) -> MecisInfo {
    let mut stmt = conn.prepare("SELECT DISTINCT organism FROM mis").unwrap();
    let v_orgs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT model FROM mis").unwrap();
    let v_models = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT inreac FROM mis").unwrap();
    let v_inreacs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT exreac FROM mis").unwrap();
    let v_exreacs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT mby FROM mis").unwrap();
    let mut v_mbys: Vec<f64> = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mbys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT mpy FROM mis").unwrap();
    let mut v_mpys: Vec<f64> = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mpys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT scen FROM mis").unwrap();
    let v_scens = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<u32>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT name FROM reactions").unwrap();
    let v_reactions = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let context = MecisInfo {
        organisms: v_orgs,
        models: v_models,
        inreacs: v_inreacs,
        exreacs: v_exreacs,
        mbys: v_mbys,
        mpys: v_mpys,
        scens: v_scens,
        reactions: v_reactions,
    };
    context
}
