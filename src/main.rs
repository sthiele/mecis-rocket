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
use rocket_contrib::Template;
use rocket_contrib::Json;
use std::cmp::Ordering;
use std::env;
use std::io::Cursor;

const ROWSPERSITE: u32 = 50;
const ROWSPERFILE: u32 = 1000;

pub fn establish_connection() -> my::Pool {
    dotenv().ok();
    let pool = my::Pool::new(env::var("DATABASE_URL").expect("DATABASE_URL must be set")).unwrap();
    pool
}

#[get("/mecis_logo")]
fn mecis_logo() -> NamedFile {
    NamedFile::open("mecis-logo.png").unwrap()
}

#[derive(Serialize)]
struct TemplateContext {
    organism: Vec<String>,
    model: Vec<String>,
    inreac: Vec<String>,
    exreac: Vec<String>,
    mby: Vec<f64>,
    mpy: Vec<f64>,
    scen: Vec<u32>,
}

#[get("/")]
fn mecis() -> Template {
    let conn = establish_connection();

    let mut stmt = conn.prepare("SELECT DISTINCT organism as bla FROM mis")
        .unwrap();
    let v_orgs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT model as bla FROM mis")
        .unwrap();
    let v_models = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT inreac as bla FROM mis")
        .unwrap();
    let v_inreacs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT exreac as bla FROM mis")
        .unwrap();
    let v_exreacs = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<String>(row.unwrap()))
        .collect();

    let mut stmt = conn.prepare("SELECT DISTINCT mby as bla FROM mis").unwrap();
    let mut v_mbys: Vec<f64> = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mbys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT mpy as bla FROM mis").unwrap();
    let mut v_mpys: Vec<f64> = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<f64>(row.unwrap()))
        .collect();
    v_mpys.sort_by(|a, b| mcmp(a, b));

    let mut stmt = conn.prepare("SELECT DISTINCT scen as bla FROM mis")
        .unwrap();
    let v_scens = stmt.execute(())
        .unwrap()
        .map(|row| my::from_row::<u32>(row.unwrap()))
        .collect();

    let context = TemplateContext {
        organism: v_orgs,
        model: v_models,
        inreac: v_inreacs,
        exreac: v_exreacs,
        mby: v_mbys,
        mpy: v_mpys,
        scen: v_scens,
    };
    Template::render("mecis", &context)
}

fn id2name(conn: &my::Pool, id: u32) -> String {
    let sql = format!("SELECT name FROM reactions WHERE mecisid={}", id);
    let mut stmt = conn.prepare(sql).unwrap();
    let mut res = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<String>(row.unwrap());
        res.push(cell);
    }
    res[0].clone()
}

fn name2id(conn: &my::Pool, name: &str) -> u32 {
    let sql = format!("SELECT mecisid FROM reactions WHERE name='{}'", name);
    let mut stmt = conn.prepare(sql).unwrap();
    let mut res = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<u32>(row.unwrap());
        res.push(cell);
    }
    res[0]
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
}


fn countcis(conn: &my::Pool, q: Query) -> (u32,u32) {
    let mut sql = create_query(&conn, q);
    sql = format!("SELECT  COUNT(*), SUM(len) FROM (SELECT DISTINCT organism,model,inreac,exreac,mby,mpy,scen,len, s FROM ({}) AS TX) AS TY", sql);

//     println!("SQL: {}", sql);

    let mut stmt = conn.prepare(sql).unwrap();
    let mut res = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<(u32,u32)>(row.unwrap());
//         println!("count {:?}", cell);
        res.push(cell);
    }

    res[0]
}

#[derive(Serialize,Debug)]
struct TemplateView {
    col_offset: u32,
    mis_offset: u32,
    end_mis: u32,
    max_mis: u32,
    max_col: u32,
    mis: Vec<String>,
}

#[get("/getcis?<q>")]
fn getcis(q: Query) -> Json<TemplateView> {
    let conn = establish_connection();
    let (max_mis, max_col) = countcis(&conn, q.clone());

    if max_mis == 0 {
        let view = 
        TemplateView {
            col_offset: 0,
            mis_offset: 0,
            end_mis: 0,
            max_mis: 0,
            max_col: 0,
            mis: vec![],
        };
        return Json(view);
    }

    let mut sql = create_query(&conn, q);

    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );

    let HARDECODEDLIMIT = 20 * ROWSPERSITE;
    sql.push_str(&format!(" LIMIT {}", HARDECODEDLIMIT));

    println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
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
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, u32)>(row.unwrap());
        let key = format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            organism, model, inreac, exreac, mby, mpy, scen
        );

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
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            } else {
                //                 res.push(old_key + "<td>" + &mis + "</td>");
                res.push(format!("{}<td>{}</td>", old_key, &mis));
                old_key = key;
                old_set_id = s;
                mis = "".to_string();
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            }
        } else {
            let name = id2name(&conn, r);
            mis.push_str(&format!("{} ", name));
        }
    }
    res.push(format!("{}<td>{}</td>", old_key, &mis));
    
    let view =   TemplateView {
        col_offset: col_counter,
        mis_offset: 1,
        end_mis: counter,
        max_mis: max_mis,
        max_col: max_col,
        mis: res,
    };
    
//     println!("view: {:?}",view);
    Json(view)
}

#[get("/getcsv?<q>")]
fn getcsv(q: Query) -> Stream<Cursor<String>> {
    let conn = establish_connection();
    let (max_mis,max_col) = countcis(&conn, q.clone());

    let mut stream = "".to_string();

    if max_mis == 0 {
        return Stream::from(Cursor::new(stream));
    }

    let mut sql = create_query(&conn, q);

    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );

    let HARDECODEDLIMIT = 20 * ROWSPERFILE;
    sql.push_str(&format!(" LIMIT {}", HARDECODEDLIMIT));

    println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;
    let tmp = stmt.execute(()).unwrap();
    for row in tmp {
//     tmp.map(|row|{
        if counter > ROWSPERFILE {
            break;
        }
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, u32)>(row.unwrap());
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
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            } else {
                stream.push_str(&format!("{},{}\n", old_key, &mis));
                old_key = key;
                old_set_id = s;
                mis = "".to_string();
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            }
        } else {
            let name = id2name(&conn, r);
            mis.push_str(&format!("{} ", name));
        }
    }
//     );
    stream.push_str(&format!("{},{}\n", old_key, &mis));
    Stream::from(Cursor::new(stream))
}

#[derive(FromForm, Clone)]
struct MoreQuery {
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
    mis_offset: u32,
    max_mis: u32,
    max_col: u32,
}

#[get("/getcism?<q>")]
fn getmorecis(q: MoreQuery) -> Json<TemplateView> {
    let conn = establish_connection();

    let qs = Query {
        organism: q.organism,
        model: q.model,
        inreac: q.inreac,
        exreac: q.exreac,
        mby: q.mby,
        mpy: q.mpy,
        scen: q.scen,
        mustin: q.mustin,
        forbidden: q.forbidden,
    };
    let mut sql = create_query(&conn, qs);

    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );

    let HARDECODEDLIMIT = 20 * ROWSPERSITE;
    sql.push_str(&format!(" LIMIT {} OFFSET {}", HARDECODEDLIMIT, q.col_offset));

    let mut stmt = conn.prepare(&sql).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;

    let mut col_counter = 0;
    for row in stmt.execute(()).unwrap() {
        if counter > ROWSPERSITE {
            break;
        }
        col_counter = col_counter + 1;
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, u32)>(row.unwrap());
        let key = format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            organism, model, inreac, exreac, mby, mpy, scen
        );
        if old_key != key || old_set_id != s {
            if counter == ROWSPERSITE {
                col_counter = col_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = key.clone();
                old_set_id = s;
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            } else {
                res.push(old_key.clone() + "<td>" + &mis + "</td>");
                old_key = key.clone();
                old_set_id = s;
                mis = "".to_string();
                let name = id2name(&conn, r);
                mis.push_str(&format!("{} ", name));
            }
        } else {
            let name = id2name(&conn, r);
            mis.push_str(&format!("{} ", name));
        }
    }
    res.push(old_key.clone() + "<td>" + &mis + "</td>");
    let view = TemplateView {
        col_offset: q.col_offset + col_counter,
        mis_offset: q.mis_offset + 1,
        end_mis: q.mis_offset + counter,
        max_mis: q.max_mis,
        max_col: q.max_col,
        mis: res,
    };
    Json(view)
}

fn create_query(conn: &my::Pool, q: Query) -> String {
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
        let mecisid = name2id(conn, r);
        outer_sql = format!(
            "{} AND EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, mecisid);
        counter = counter + 1;
    }

    while let Some(r) = forbidden.next() {
        let mecisid = name2id(conn, r);
        outer_sql = format!(
            "{} AND NOT EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, mecisid);
        counter = counter + 1;
    }

    outer_sql
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![mecis])
        .mount("/", routes![getcis])
        .mount("/", routes![getcsv])
        .mount("/", routes![getmorecis])
        .mount("/", routes![mecis_logo])
        .attach(Template::fairing())
    //         .catch(errors![not_found])
}

fn main() {
    rocket().launch();
}

