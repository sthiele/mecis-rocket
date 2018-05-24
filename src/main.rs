#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate rusqlite;

#[macro_use]
extern crate serde_derive;
use rocket_contrib::Template;
use rusqlite::Connection;

use rocket::response::NamedFile;

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
    let conn = Connection::open("mecis.db").unwrap();

    let mut stmt = conn.prepare("SELECT DISTINCT organism as bla FROM mis")
        .unwrap();
    let mut organisms = stmt.query_map(&[], |row| row.get::<_, String>(0)).unwrap();

    let mut v_orgs = vec![];
    while let Some(result_row) = organisms.next() {
        let row = result_row.unwrap();
        v_orgs.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT model as bla FROM mis")
        .unwrap();
    let mut models = stmt.query_map(&[], |row| row.get::<_, String>(0)).unwrap();

    let mut v_models = vec![];
    while let Some(result_row) = models.next() {
        let row = result_row.unwrap();
        v_models.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT inreac as bla FROM mis")
        .unwrap();
    let mut inreacs = stmt.query_map(&[], |row| row.get::<_, String>(0)).unwrap();

    let mut v_inreacs = vec![];
    while let Some(result_row) = inreacs.next() {
        let row = result_row.unwrap();
        v_inreacs.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT exreac as bla FROM mis")
        .unwrap();
    let mut exreacs = stmt.query_map(&[], |row| row.get::<_, String>(0)).unwrap();

    let mut v_exreacs = vec![];
    while let Some(result_row) = exreacs.next() {
        let row = result_row.unwrap();
        v_exreacs.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT mby as bla FROM mis").unwrap();
    let mut mbys = stmt.query_map(&[], |row| row.get::<_, f64>(0)).unwrap();

    let mut v_mbys = vec![];
    while let Some(result_row) = mbys.next() {
        let row = result_row.unwrap();
        v_mbys.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT mpy as bla FROM mis").unwrap();
    let mut mpys = stmt.query_map(&[], |row| row.get::<_, f64>(0)).unwrap();

    let mut v_mpys = vec![];
    while let Some(result_row) = mpys.next() {
        let row = result_row.unwrap();
        v_mpys.push(row);
    }

    let mut stmt = conn.prepare("SELECT DISTINCT scen as bla FROM mis")
        .unwrap();
    let mut scens = stmt.query_map(&[], |row| row.get::<_, u32>(0)).unwrap();

    let mut v_scens = vec![];
    while let Some(result_row) = scens.next() {
        let row = result_row.unwrap();
        v_scens.push(row);
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
#[get("/countcis/<organism>/<model>/<inreac>/<exreac>/<mby>/<mpy>/<scen>")]
fn s_countcis(
    organism: String,
    model: String,
    inreac: String,
    exreac: String,
    mby: f64,
    mpy: f64,
    scen: u32,
) -> String {
    format!(
        "<br>{} intervention sets found!",
        countcis(&organism, &model, &inreac, &exreac, mby, mpy, scen,)
    )
}

fn countcis(
    organism: &str,
    model: &str,
    inreac: &str,
    exreac: &str,
    mby: f64,
    mpy: f64,
    scen: u32,
) -> u32 {
    let conn = Connection::open("mecis.db").unwrap();

    let mustin = vec!["85".to_string(), "512".to_string(), "925".to_string()];
    let forbidden = vec!["1226".to_string(), "1227".to_string()];
    let mut sql = create_query(&organism,
        &model,
        &inreac,
        &exreac,
        &mby,
        &mpy,
        &scen,
        &mustin,
        &forbidden
    );

    sql = format!("SELECT COUNT(*) FROM ({})", sql);
    let mut stmt = conn.prepare(&sql).unwrap();
    let mut count = stmt.query_map(&[], |row| row.get::<_, u32>(0)).unwrap();
    let mut res = vec![];
    while let Some(result_row) = count.next() {
        let row = result_row.unwrap();
//         println!("count {}", row);
        res.push(row);
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

#[derive(Debug)]
struct Bla {
    key: String,
    set_id: u32,
    reac_id: u32,
}

// #[get("/getcis/<organism>/<model>/<inreac>/<exreac>/<mby>/<mpy>/<scen>/<MUSTIN>")]
#[get("/getcis/<organism>/<model>/<inreac>/<exreac>/<mby>/<mpy>/<scen>")]
fn getcis(
    organism: String,
    model: String,
    inreac: String,
    exreac: String,
    mby: f64,
    mpy: f64,
    scen: u32,
    //     mustin: String,
) -> Template {
    let num_sets = countcis(&organism, &model, &inreac, &exreac, mby, mpy, scen);

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

    let conn = Connection::open("mecis.db").unwrap();

    let mustin = vec!["85".to_string(), "512".to_string(), "925".to_string()];
    let forbidden = vec!["1226".to_string(), "1227".to_string()];
    let mut sql = create_query(&organism,
        &model,
        &inreac,
        &exreac,
        &mby,
        &mpy,
        &scen,
        &mustin,
        &forbidden
    );

    sql = format!("SELECT organism, p1, p2, p3, p4, p5, p6, p7, r FROM mis inner join ({}) ON p1=model AND p2=inreac AND p3=exreac AND p4=mby AND p5=mpy AND p6=scen AND p7=s", sql);

    let mut stmt = conn.prepare(&sql).unwrap();
    let mut intervention_sets = stmt.query_map(&[], |row| {
        let organism = row.get::<_, String>(0);
        let model = row.get::<_, String>(1);
        let inreac = row.get::<_, String>(2);
        let exreac = row.get::<_, String>(3);
        let mby = row.get::<_, f64>(4);
        let mpy = row.get::<_, f64>(5);
        let scen = row.get::<_, u32>(6);
        Bla {
            key: format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                organism, model, inreac, exreac, mby, mpy, scen
            ),
            set_id: row.get::<_, u32>(7),
            reac_id: row.get::<_, u32>(8),
        }
    }).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;
    let mut sql_counter = 0;
    let rowspersite = 100;
    while let Some(result_row) = intervention_sets.next() {
        sql_counter = sql_counter + 1;
        let row = result_row.unwrap();
        if old_key != row.key || old_set_id != row.set_id {
            if counter == rowspersite {
                sql_counter = sql_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = row.key.clone();
                old_set_id = row.set_id;
                mis.push_str(&format!("{} ", row.reac_id));
            } else {
                res.push(old_key.clone() + "<td>" + &mis + "</td>");
                old_key = row.key.clone();
                old_set_id = row.set_id;
                mis = "".to_string();
                mis.push_str(&format!("{} ", row.reac_id));
            }
        } else {
            mis.push_str(&format!("{} ", row.reac_id));
        }
    }
    res.push(old_key.clone() + "<td>" + &mis + "</td>");

    let view = TemplateView {
        sql_offset: sql_counter,
        start_mis: 1,
        end_mis: counter,
        max_mis: num_sets,
        mis: res,
    };
    Template::render("view", &view)
}

fn create_query(organism: &str,
    model: &str,
    inreac: &str,
    exreac: &str,
    mby: &f64,
    mpy: &f64,
    scen: &u32, mustin: &Vec<String>, forbidden: &Vec<String>) -> String {

    let mut sql = "SELECT DISTINCT model as p1, inreac as p2, exreac as p3, mby as p4, mpy as p5, scen as p6, s as p7  FROM mis WHERE 1 ".to_string();
    fix_parameters(
        &mut sql,
        &organism,
        &model,
        &inreac,
        &exreac,
        &mby,
        &mpy,
        &scen,
    );

    for r in mustin {
        sql = format!(
        "{} AND EXISTS (SELECT r FROM mis WHERE r ='{}' AND model=p1 AND inreac=p2 AND exreac=p3 AND mby=p4 AND mpy=p5 AND scen=p6 AND s=p7)", sql, r);
    }

    for r in forbidden {
        sql = format!(
        "{} AND NOT EXISTS (SELECT r FROM mis WHERE r ='{}' AND model=p1 AND inreac=p2 AND exreac=p3 AND mby=p4 AND mpy=p5 AND scen=p6 AND s=p7)", sql, r);
    }
    println!("{}",sql);
    sql
}
fn fix_parameters(
    sql: &mut String,
    organism: &str,
    model: &str,
    inreac: &str,
    exreac: &str,
    mby: &f64,
    mpy: &f64,
    scen: &u32,
) {
    if organism != "None" {
        sql.push_str(" AND organism='");
        sql.push_str(organism);
        sql.push('\'');
    }
    if model != "None" {
        sql.push_str(" AND model='");
        sql.push_str(model);
        sql.push('\'');
    }
    if inreac != "None" {
        sql.push_str(" AND inreac='");
        sql.push_str(inreac);
        sql.push('\'');
    }
    if exreac != "None" {
        sql.push_str(" AND exreac='");
        sql.push_str(exreac);
        sql.push('\'');
    }
    if !mby.is_nan() {
        sql.push_str(" AND mby='");
        sql.push_str(&format!("{}", mby));
        sql.push('\'');
    }
    if !mpy.is_nan() {
        sql.push_str(" AND mpy='");
        sql.push_str(&format!("{}", mpy));
        sql.push('\'');
    }
    sql.push_str(" AND scen='");
    sql.push_str(&format!("{}", scen));
    sql.push('\'');
}

#[get("/getcis/<organism>/<model>/<inreac>/<exreac>/<mby>/<mpy>/<scen>/<offset>/<mis_offset>/<num_sets>")]
fn getmorecis(
    organism: String,
    model: String,
    inreac: String,
    exreac: String,
    mby: f64,
    mpy: f64,
    scen: u32,
    offset: u32,
    mis_offset: u32,
    num_sets: u32,
) -> Template {
    let conn = Connection::open("mecis.db").unwrap();

    let mustin = vec!["85".to_string(), "512".to_string(), "925".to_string()];
    let forbidden = vec!["1226".to_string(), "1227".to_string()];
    let mut sql = create_query(&organism,
        &model,
        &inreac,
        &exreac,
        &mby,
        &mpy,
        &scen,
        &mustin,
        &forbidden
    );

    sql = format!("SELECT organism, p1, p2, p3, p4, p5, p6, p7, r FROM mis inner join ({}) ON p1=model AND p2=inreac AND p3=exreac AND p4=mby AND p5=mpy AND p6=scen AND p7=s", sql);

    let HARDECODEDLIMIT = 20 * 50;
    sql.push_str(&format!(" LIMIT {} OFFSET {}", HARDECODEDLIMIT, offset));

    let mut stmt = conn.prepare(&sql).unwrap();
    let mut intervention_sets = stmt.query_map(&[], |row| {
        let organism = row.get::<_, String>(0);
        let model = row.get::<_, String>(1);
        let inreac = row.get::<_, String>(2);
        let exreac = row.get::<_, String>(3);
        let mby = row.get::<_, f64>(4);
        let mpy = row.get::<_, f64>(5);
        let scen = row.get::<_, u32>(6);
        Bla {
            key: format!(
                "<td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td>",
                organism, model, inreac, exreac, mby, mpy, scen
            ),
            set_id: row.get::<_, u32>(7),
            reac_id: row.get::<_, u32>(8),
        }
    }).unwrap();

    let mut res = vec![];
    let mut mis = "".to_string();
    let mut old_key = "".to_string();
    let mut old_set_id = 0;
    let mut first = true;
    let mut counter = 0;

    let mut sql_counter = 0;
    while let Some(result_row) = intervention_sets.next() {
        sql_counter = sql_counter + 1;
        let row = result_row.unwrap();
        if old_key != row.key || old_set_id != row.set_id {
            if counter == 50 {
                sql_counter = sql_counter - 1;
                break;
            }
            counter = counter + 1;
            if first {
                first = false;
                old_key = row.key.clone();
                old_set_id = row.set_id;
                mis.push_str(&format!("{} ", row.reac_id));
            } else {
                res.push(old_key.clone() + "<td>" + &mis + "</td>");
                old_key = row.key.clone();
                old_set_id = row.set_id;
                mis = "".to_string();
                mis.push_str(&format!("{} ", row.reac_id));
            }
        } else {
            mis.push_str(&format!("{} ", row.reac_id));
        }
    }
    res.push(old_key.clone() + "<td>" + &mis + "</td>");

    let view = TemplateView {
        sql_offset: offset + sql_counter,
        start_mis: mis_offset + 1,
        end_mis: mis_offset + counter,
        max_mis: num_sets,
        mis: res,
    };
    Template::render("view", &view)
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
