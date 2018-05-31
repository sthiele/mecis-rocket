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
use rocket_contrib::Template;
use std::env;

const ROWSPERSITE: u32 = 50;

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
    let mut v_orgs = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<String>(row.unwrap());
        v_orgs.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT model as bla FROM mis")
        .unwrap();

    let mut v_models = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<String>(row.unwrap());
        v_models.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT inreac as bla FROM mis")
        .unwrap();

    let mut v_inreacs = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<String>(row.unwrap());
        v_inreacs.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT exreac as bla FROM mis")
        .unwrap();

    let mut v_exreacs = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<String>(row.unwrap());
        v_exreacs.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT mby as bla FROM mis").unwrap();

    let mut v_mbys = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<f64>(row.unwrap());
        v_mbys.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT mpy as bla FROM mis").unwrap();

    let mut v_mpys = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<f64>(row.unwrap());
        v_mpys.push(cell);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT scen as bla FROM mis")
        .unwrap();

    let mut v_scens = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<u32>(row.unwrap());
        v_scens.push(cell);
    }

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

#[get("/countcis?<q>")]
fn s_countcis(q: Query) -> String {
    format!("<br>{} intervention sets found!", countcis(q))
}

fn countcis(q: Query) -> u32 {
    let conn = establish_connection();

    let mut sql = create_query(q);

    sql = format!("SELECT COUNT(*) FROM (SELECT DISTINCT organism,model,inreac,exreac,mby,mpy,scen,s FROM ({}) AS TX) AS TY", sql);
    println!("SQL: {}", sql);

    let mut stmt = conn.prepare(sql).unwrap();
    let mut res = vec![];
    for row in stmt.execute(()).unwrap() {
        let cell = my::from_row::<u32>(row.unwrap());
        println!("count {}", cell);
        res.push(cell);
    }

    res[0]
}

#[derive(Serialize)]
struct TemplateView {
    sql_offset: u32,
    start_mis: u32,
    end_mis: u32,
    max_mis: u32,
    mis: Vec<String>,
}

#[get("/getcis?<q>")]
fn getcis(q: Query) -> Template {
    let num_sets = countcis(q.clone());

    if num_sets == 0 {
        let view = TemplateView {
            sql_offset: 0,
            start_mis: 0,
            end_mis: 0,
            max_mis: 0,
            mis: vec![],
        };
        return Template::render("view", &view);
    }

    let conn = establish_connection();

    let mut sql = create_query(q);

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
    let mut sql_counter = 0;
    let tmp = stmt.execute(()).unwrap();
    for row in tmp {
        sql_counter = sql_counter + 1;
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, u32)>(row.unwrap());
        let key = format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            organism, model, inreac, exreac, mby, mpy, scen
        );

        if old_key != key || old_set_id != s {
            if counter == ROWSPERSITE {
                sql_counter = sql_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = key;
                old_set_id = s;
                mis.push_str(&format!("{} ", r));
            } else {
                //                 res.push(old_key + "<td>" + &mis + "</td>");
                res.push(format!("{}<td>{}</td>", old_key, &mis));
                old_key = key;
                old_set_id = s;
                mis = "".to_string();
                mis.push_str(&format!("{} ", r));
            }
        } else {
            mis.push_str(&format!("{} ", r));
        }
    }
    //     res.push(old_key + "<td>" + &mis + "</td>");

    res.push(format!("{}<td>{}</td>", old_key, &mis));
    let view = TemplateView {
        sql_offset: sql_counter,
        start_mis: 1,
        end_mis: counter,
        max_mis: num_sets,
        mis: res,
    };
    Template::render("view", &view)
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
    offset: u32,
    mis_offset: u32,
    num_sets: u32,
}

#[get("/getcism?<q>")]
fn getmorecis(q: MoreQuery) -> Template {
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
    let mut sql = create_query(qs);

    sql = format!(
        "SELECT organism, model, inreac, exreac, mby, mpy, scen, s, r FROM ({}) AS TY",
        sql
    );

    let HARDECODEDLIMIT = 20 * ROWSPERSITE;
    sql.push_str(&format!(" LIMIT {} OFFSET {}", HARDECODEDLIMIT, q.offset));

    let mut stmt = conn.prepare(&sql).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;

    let mut sql_counter = 0;
    for row in stmt.execute(()).unwrap() {
        sql_counter = sql_counter + 1;
        let (organism, model, inreac, exreac, mby, mpy, scen, s, r) =
            my::from_row::<(String, String, String, String, f64, f64, u32, u32, u32)>(row.unwrap());
        let key = format!(
            "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
            organism, model, inreac, exreac, mby, mpy, scen
        );
        if old_key != key || old_set_id != s {
            if counter == ROWSPERSITE {
                sql_counter = sql_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = key.clone();
                old_set_id = s;
                mis.push_str(&format!("{} ", r));
            } else {
                res.push(old_key.clone() + "<td>" + &mis + "</td>");
                old_key = key.clone();
                old_set_id = s;
                mis = "".to_string();
                mis.push_str(&format!("{} ", r));
            }
        } else {
            mis.push_str(&format!("{} ", r));
        }
    }
    res.push(old_key.clone() + "<td>" + &mis + "</td>");
    let view = TemplateView {
        sql_offset: q.offset + sql_counter,
        start_mis: q.mis_offset + 1,
        end_mis: q.mis_offset + counter,
        max_mis: q.num_sets,
        mis: res,
    };
    Template::render("view", &view)
}

fn create_query(q: Query) -> String {
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
        outer_sql = format!(
            "{} AND EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, r);
        counter = counter + 1;
    }

    while let Some(r) = forbidden.next() {
        outer_sql = format!(
            "{} AND NOT EXISTS (SELECT r FROM ({}) AS T{} WHERE r ='{}' AND model=T0.model AND inreac=T0.inreac AND exreac=T0.exreac AND mby=T0.mby AND mpy=T0.mpy AND scen=T0.scen AND s=T0.s)", outer_sql, sql,counter, r);
        counter = counter + 1;
    }

    outer_sql
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![mecis])
        .mount("/", routes![s_countcis])
        .mount("/", routes![getcis])
        .mount("/", routes![getmorecis])
        .mount("/", routes![mecis_logo])
        .attach(Template::fairing())
    //         .catch(errors![not_found])
}

fn main() {
    rocket().launch();
}
