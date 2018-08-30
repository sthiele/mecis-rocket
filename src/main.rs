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

#[get("/favicon.ico")]
fn favicon() -> NamedFile {
    NamedFile::open("frontend/favicon.ico").unwrap()
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
    mapping.push(KnockOut {
        name: "Error".to_string(),
        link: None,
    });
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
    sql = format!("SELECT  COUNT(*) FROM ({}) AS TY", sql);
//     println!("SQL: {}", sql);
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
    let mut rows = vec![];
    let mut col_counter = 0;
    let mapping = &st.mapping;
    let max_mis;
    if q.col_offset == 0 {
        max_mis = countcis(&conn, q.clone());
        if max_mis == 0 {
            let view = QueryResponse {
                col_offset: 0,
                max_mis: 0,
                rows: rows,
            };
            return Json(view);
        }
    } else {
        max_mis = 0;
    }

    let mut sql = create_query(&conn, &q);
    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, set_id FROM ({} LIMIT {} OFFSET {}) AS TY",
        sql, ROWSPERSITE, q.col_offset
    );
//     println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();
    let tmp = stmt.execute(()).unwrap();

    for row in tmp {
        let (organism, model, inreac, exreac, mby, mpy, scen, set_id) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32)>(row.unwrap());
        sql = format!("SELECT r FROM interventionsets WHERE set_id='{}'", set_id);
//         println!("SQL: {}", sql);
        let mut stmt = conn.prepare(&sql).unwrap();
        let tmp2 = stmt.execute(()).unwrap();
        let mut mis = vec![];
        for row2 in tmp2 {
            let r = my::from_row::<usize>(row2.unwrap());
            let ko = &mapping[r];
            mis.push(ko.clone());
        }
        let response = ResponseRoW {
            organism: organism,
            model: model,
            inreac: inreac,
            exreac: exreac,
            mby: mby,
            mpy: mpy,
            scen: scen,
            mis: mis,
        };
        col_counter = col_counter + 1;
        rows.push(response.clone());
    }

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
    let mut stream = "".to_string();
    let mapping = &st.mapping;
    let max_mis = countcis(&conn, q.clone());

    if max_mis == 0 {
        return Stream::from(Cursor::new(stream));
    }

    let mut sql = create_query(&conn, &q);
    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, set_id FROM ({} LIMIT {} ) AS TY",
        sql, ROWSPERFILE
    );
//     println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let tmp = stmt.execute(()).unwrap();

    for row in tmp {
        let (organism, model, inreac, exreac, mby, mpy, scen, set_id) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32)>(row.unwrap());
        sql = format!("SELECT r FROM interventionsets WHERE set_id='{}'", set_id);
//         println!("SQL: {}", sql);
        let mut stmt = conn.prepare(&sql).unwrap();
        let tmp2 = stmt.execute(()).unwrap();
        let mut mis = "".to_string();
        for row2 in tmp2 {
            let r = my::from_row::<usize>(row2.unwrap());
            let ko = &mapping[r];
            mis.push_str(&format!("{} ", ko.name));
        }

        stream.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            organism, model, inreac, exreac, mby, mpy, scen, &mis
        ));
    }

    Stream::from(Cursor::new(stream))
}

fn create_query(conn: &my::Pool, q: &Query) -> String {
    let mut sql = "SELECT mis_short.organism, mis_short.model, mis_short.inreac, mis_short.exreac, mis_short.mby, mis_short.mpy, mis_short.scen, mis_short.set_id FROM mis_short WHERE 1".to_string();
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

    let mut outer_sql = sql;

    let mut mustin = q.mustin.split_whitespace();
    let mut forbidden = q.forbidden.split_whitespace();
    let mut counter = 1;

    while let Some(r) = mustin.next() {
        if let Some(mecisid) = name2id(conn, r) {
            let r_sql = format!("SELECT set_id FROM interventionsets WHERE r ='{}'", mecisid);
            outer_sql = format!(
            "SELECT T{c1}.organism, T{c1}.model, T{c1}.inreac, T{c1}.exreac, T{c1}.mby, T{c1}.mpy, T{c1}.scen, T{c1}.set_id FROM ({left}) as T{c1} JOIN ({right}) as T{c2} on T{c1}.set_id=T{c2}.set_id", left=outer_sql, right=r_sql, c1=counter,c2=counter+1);

            counter = counter + 2;
        } else {
            outer_sql = format!("{} AND 1=0", outer_sql);
        }
    }

    while let Some(r) = forbidden.next() {
        if let Some(mecisid) = name2id(conn, r) {
            let r_sql = format!("SELECT set_id FROM interventionsets WHERE r ='{}'", mecisid);
            outer_sql = format!(
            "SELECT T{c1}.organism, T{c1}.model, T{c1}.inreac, T{c1}.exreac, T{c1}.mby, T{c1}.mpy, T{c1}.scen, T{c1}.set_id FROM ({left}) as T{c1} LEFT JOIN ({right}) as T{c2} on T{c1}.set_id=T{c2}.set_id WHERE T{c2}.set_id IS NULL",left=outer_sql, right=r_sql, c1=counter,c2=counter+1);
            counter = counter + 2;
        }
    }

    outer_sql
}
#[catch(404)]
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
        .mount("/", routes![favicon])
        .catch(catchers![not_found])
}

fn main() {
    rocket().launch();
}

fn info(conn: &my::Pool) -> MecisInfo {
    let mut stmt = conn.prepare("SELECT name FROM organisms").unwrap();
    let v_orgs = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT name FROM models").unwrap();
    let v_models = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT name FROM inreacs").unwrap();
    let v_inreacs = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT name FROM exreacs").unwrap();
    let v_exreacs = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT mby FROM mis_short").unwrap();
    let mut v_mbys: Vec<f64> = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mbys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT mpy FROM mis_short").unwrap();
    let mut v_mpys: Vec<f64> = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mpys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT scen FROM mis_short").unwrap();
    let v_scens = stmt
        .execute(())
        .unwrap()
        .map(|row| my::from_row::<u32>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT name FROM reactions").unwrap();
    let v_reactions = stmt
        .execute(())
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
