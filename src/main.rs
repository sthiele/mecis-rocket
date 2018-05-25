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
use std::str::SplitWhitespace;


const ROWSPERSITE: u32 = 10;

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
    forbidden: String
}

#[get("/countcis?<q>")]
fn s_countcis(
    q: Query,
) -> String {
    format!(
        "<br>{} intervention sets found!",
        countcis(q)
    )
}

fn countcis(
    q : Query
) -> u32 {

    let conn = establish_connection();

    let sql = create_query(q);

    //     sql = format!("SELECT COUNT(*) FROM ({}) AS TX", sql);
    //     println!("SQL: {}",sql);

    let mut stmt = conn.prepare(sql).unwrap();
    //     let mut res = vec![];
    let mut count = 0;
    for row in stmt.execute(()).unwrap() {
        //         let cell = my::from_row::<u32>(row.unwrap());
        //         println!("count {}", cell);
        //         res.push(cell);
        count = count + 1;
    }

    //     res[0]
    count
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
fn getcis(
    q: Query
) -> Template {

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

    sql = format!("SELECT organism, p1, p2, p3, p4, p5, p6, p7, r FROM mis inner join ({}) AS TX ON p1=model AND p2=inreac AND p3=exreac AND p4=mby AND p5=mpy AND p6=scen AND p7=s", sql);

//     println!("SQL: {}", sql);
    let mut stmt = conn.prepare(&sql).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;
    let mut sql_counter = 0;
println!("hi1");
    let tmp =  stmt.execute(()).unwrap();  
println!("hi2");
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
                res.push(format!("{}<td>{}</td>",old_key, &mis));
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
    
                res.push(format!("{}<td>{}</td>",old_key, &mis));
println!("hi3");
    let view = TemplateView {
        sql_offset: sql_counter,
        start_mis: 1,
        end_mis: counter,
        max_mis: num_sets,
        mis: res,
    };
println!("hi4");
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
    num_sets: u32
}

#[get("/getcism?<q>")]
fn getmorecis(q: MoreQuery
) -> Template {

    let conn = establish_connection();

    let qs = Query {
        organism: q.organism,
        model: q.model,
        inreac: q.inreac,
        exreac: q.exreac,
        mby: q.mby,
        mpy: q.mpy,
        scen: q.scen,
        mustin : q.mustin,
        forbidden : q.forbidden,
    };
    let mut sql = create_query(qs );

    sql = format!("SELECT organism, p1, p2, p3, p4, p5, p6, p7, r FROM mis inner join ({}) AS TX ON p1=model AND p2=inreac AND p3=exreac AND p4=mby AND p5=mpy AND p6=scen AND p7=s", sql);

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

fn create_query(
    q: Query,
) -> String {
    let mut sql = "SELECT DISTINCT model as p1, inreac as p2, exreac as p3, mby as p4, mpy as p5, scen as p6, s as p7  FROM mis WHERE 1 ".to_string();
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
    
    let mut mustin= q.mustin.split_whitespace();
    let mut forbidden = q.forbidden.split_whitespace();
        
    let mut count = 0;
    while let Some(r) = mustin.next() {
        let mut sql1 = format!(
        "SELECT DISTINCT model as q1, inreac as q2, exreac as q3, mby as q4, mpy as q5, scen as q6, s as q7  FROM mis WHERE r ='{}'",r);

        sql = format!(
          "SELECT p1, p2, p3, p4, p5, p6, p7 FROM ({}) AS T{} inner join ({}) AS T{} ON p1=q1 AND p2=q2 AND p3=q3 AND p4=q4 AND p5=q5 AND p6=q6 AND p7=q7",sql,count,sql1,count+1);
        count = count + 2
    }

    if let Some(r) = forbidden.nth(0)  {
        sql = format!(
            "SELECT p1, p2, p3, p4, p5, p6, p7  FROM ({}) AS T{} WHERE 1 ",
            sql, count
        );
        sql = format!(
            "{} AND NOT EXISTS (SELECT r FROM mis WHERE r ='{}' AND model=p1 AND inreac=p2 AND exreac=p3 AND mby=p4 AND mpy=p5 AND scen=p6 AND s=p7)", sql, r);
            
        while let Some(r) = forbidden.next() {
            sql = format!(
            "{} AND NOT EXISTS (SELECT r FROM mis WHERE r ='{}' AND model=p1 AND inreac=p2 AND exreac=p3 AND mby=p4 AND mpy=p5 AND scen=p6 AND s=p7)", sql, r);
        }
    }
    //     println!("{}",sql);
    sql
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
