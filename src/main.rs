use std::fs;
use scraper::{Html, Selector, ElementRef};
use std::string::ToString;
use strum_macros;
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use std::collections::HashMap;

use rocket::{get, post, routes};
use rocket::http::{Header};

#[macro_use] extern crate rocket;
use rocket::request::Request;
use rocket::response::{Response};


use rocket::fairing::{Fairing, Info, Kind};

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        println!("Setting access control allow origin");
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PUT, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));

    }
}


#[get("/nba/box/<team_code>")]
async fn box_score(team_code: &str) -> Json<TeamBox> {
    let team_box = get_team_box_score(team_code).await;
    return Json(team_box);
}

#[get("/nba/upcoming-probable-lineup/<team_code>")]
async fn get_probable_lineups(team_code: String)  -> Json<Vec<ProbableLineup>>  {
    let team_box_score = get_team_box_score(&team_code).await;

    let team_page_html = reqwest::get(format!("https://www.espn.com/nba/team/_/name/{}", team_code)).await.unwrap().text().await;
    let opponent_team_code = get_upcoming_opponent_team_code(team_page_html.unwrap());
    let opponent_team_box_score = get_team_box_score(&opponent_team_code).await;

    let injuries = get_injuries_with_team_code().await;

    let team_probable_lineup = ProbableLineup {
        team_code: team_code.to_owned(),
        lineup_by_position: probable_lineups(&team_box_score.player_records),
        injury_report: injuries.to_owned().into_iter().find(|tij| tij.team_code == team_code).unwrap()
    };

    let opponent_team_probable_lineup = ProbableLineup {
        team_code: opponent_team_code.to_owned(),
        lineup_by_position: probable_lineups(&opponent_team_box_score.player_records),
        injury_report: injuries.to_owned().into_iter().find(|tij| tij.team_code == opponent_team_code).unwrap()
    };


    return Json(vec![team_probable_lineup, opponent_team_probable_lineup]);

}

async fn get_team_box_score(team_code: &str) -> TeamBox {
    let team_page_html = reqwest::get(format!("https://www.espn.com/nba/team/_/name/{}", team_code)).await.unwrap().text().await.unwrap();
    let latest_game_id = get_latest_game_id(team_page_html); // 401307777
    let boxscore_page_html = reqwest::get(format!("https://www.espn.com/nba/boxscore/_/gameId/{}", latest_game_id)).await.unwrap().text().await.unwrap();
    let team_box = get_latest_game_box(&boxscore_page_html, get_orientation(&boxscore_page_html, team_code));
    team_box
}

#[post("/teams")]
async fn teams() -> Json<Vec<Team>> {
    return Json(get_teams(reqwest::get("https://www.espn.com/nba/teams").await.unwrap().text().await.unwrap()));
}

#[get("/injuries")]
async fn get_injuries() -> Json<Vec<TeamInjuryReport>> {
    return Json(get_injuries_with_team_code().await);
}

async fn get_injuries_with_team_code() -> Vec<TeamInjuryReport> {
    let teams = get_teams(reqwest::get("https://www.espn.com/nba/teams").await.unwrap().text().await.unwrap());
    let team_injury_reports = injuries(reqwest::get("https://www.espn.com/nba/injuries").await.unwrap().text().await.unwrap());
    let mut team_injury_reports_return = Vec::new();
    for team in &teams {
        for tir in team_injury_reports.to_owned() {
            if team.full_name == tir.team_name {
                team_injury_reports_return.push(TeamInjuryReport {
                    team_code: team.id.to_string(),
                    team_name: tir.team_name.clone(),
                    injuries: tir.injuries.to_owned()
                });
            }
        }
    }
    return team_injury_reports_return;
}


#[launch]
fn rocket() -> _ {
    rocket::build().attach(CORS).mount("/", routes![box_score, teams, get_probable_lineups, get_injuries])
}


#[derive(strum_macros::ToString, Debug)]
#[allow(non_camel_case_types)]
enum HomeOrAway {
    home,
    away,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamBox {
    pub overview: Overview,
    pub player_records: Vec<Player>,
    pub manager: HashMap<String, String>

}
// has of a hash

#[derive(Debug, Serialize, Deserialize)]
pub struct Overview {
    score: GameScore,
    event: TwoTeams,
    share_url: String
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TwoTeams {
    away_team: OrientedTeam,
    home_team: OrientedTeam
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Logos {
    w72xh72: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrientedTeam {
    logos: Logos,
    id: String,
    medium_name: String
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerInjury {
    name: String,
    date: String,
    position: String,
    status: String,
    description: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamInjuryReport {
    team_code: String,
    team_name: String,
    injuries: Vec<PlayerInjury>
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerBoxScore {
    starter: bool,
    first_initial_and_last_name: String,
    player_id: String,
    position: String,
    minutes: String,
    field_goals_made: String,
    field_goals_attempted: String,
    three_point_field_goals_made: String,
    three_point_field_goals_attempted: String,
    free_throws_made: String,
    free_throws_attempted: String,
    oreb: String,
    dreb: String,
    rebounds_total: String,
    assists: String,
    steals: String,
    blocked_shots: String,
    turnovers: String,
    pf: String,
    plus_minus: String,
    points: String,
    dnp: String,
    headshots: HashMap<String, String>
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PositionOptions {
    position: String,
    players: Vec<Player>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Team {
    id: String,
    full_name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    id: String,
    alignment: String,
    player: PlayerBoxScore
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameScore {
    away: TeamScore,
    home: TeamScore
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamScore {
    score: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProbableLineup {
    team_code: String,
    lineup_by_position: HashMap<String, Vec<Player>>,
    injury_report: TeamInjuryReport

}

fn get_upcoming_opponent_team_code(html: String) -> String {
    let fragment = Html::parse_fragment(&html);
    let upcoming_selector = Selector::parse("section.club-schedule ul ul li a.upcoming div.logo img").unwrap();
    let upcoming = fragment.select(&upcoming_selector).next();
    let a = upcoming.unwrap();
    return a.value().attr("src").unwrap().split("/").collect::<Vec<&str>>()[10].to_string().split(".").collect::<Vec<&str>>()[0].to_string();
}

fn get_latest_game_id(html: String) -> String {
    let fragment = Html::parse_fragment(&html);
    let last_completed_selector = Selector::parse("section.club-schedule ul ul li a:not(.upcoming)").unwrap();
    let live_selector = Selector::parse("section.club-schedule ul ul li a[rel=nbagamecast]").unwrap();
    let completed = fragment.select(&last_completed_selector).next();
    let live = fragment.select(&live_selector).next();
    let a;
    if live.is_none() {
        a = completed.unwrap();
    } else {
        a = live.unwrap();
    }
    let href = a.value().attr("href").unwrap();
    let is_game_live = href.contains("=");
    return match is_game_live {
        true => a.value().attr("href").unwrap().split("=").collect::<Vec<&str>>()[1].to_string(),
        false => a.value().attr("href").unwrap().split("/").collect::<Vec<&str>>()[5].to_string()
    };
}

fn get_game_header(html: &String) -> Overview {
    let mut score = GameScore { away: TeamScore { score: "".to_string() }, home: TeamScore { score: "".to_string() } };

    let fragment = Html::parse_fragment(&html);

    let home_selector = Selector::parse(".competitors .home").unwrap();
    let home_elem = fragment.select(&home_selector).next().unwrap();

    let box_score_link_selector = Selector::parse("link[rel=canonical").unwrap();
    let name_selector = Selector::parse(".short-name").unwrap();
    let score_selector = Selector::parse(".score").unwrap();
    let team_logo_selector = Selector::parse(".team-logo").unwrap();
    let id_selector = Selector::parse("a.team-name").unwrap();

    let mut home_oriented = OrientedTeam {
        logos: Logos { w72xh72: "".to_string() },
        id: "".to_string(),
        medium_name: "".to_string()
    };

    let mut away_oriented = OrientedTeam {
        logos: Logos { w72xh72: "".to_string() },
        id: "".to_string(),
        medium_name: "".to_string()
    };

    home_oriented.medium_name = get_first_text_value(home_elem, &name_selector);
    home_oriented.id = extract_team_code_from_a_tag(&id_selector, home_elem);
    home_oriented.logos.w72xh72 = get_src_from_img(home_elem, &team_logo_selector);
    score.home.score = get_first_text_value(home_elem, &score_selector);

    let away_selector = Selector::parse(".competitors .away").unwrap();
    let away_elem = fragment.select(&away_selector).next().unwrap();

    away_oriented.medium_name = get_first_text_value(away_elem, &name_selector);
    away_oriented.id = extract_team_code_from_a_tag(&id_selector, away_elem);
    away_oriented.logos.w72xh72 = get_src_from_img(away_elem, &team_logo_selector);
    score.away.score = get_first_text_value(away_elem, &score_selector);

    let box_score_link = fragment.select(&box_score_link_selector).next().unwrap().value().attr("href").unwrap().to_string();


    return Overview {
        share_url: box_score_link,
        score: score,
        event: TwoTeams {
            away_team: away_oriented,
            home_team: home_oriented
        }
    }

}

fn get_src_from_img(parent_element: ElementRef, team_logo_selector: &Selector) -> String {
    parent_element.select(&team_logo_selector).next().unwrap().value().attr("src").unwrap().to_string()
}

fn get_first_text_value(parent_element: ElementRef, selector: &Selector) -> String {
    let vec = parent_element.select(&selector).next().unwrap().text().collect::<Vec<_>>();
    return match vec.len() {
        0 => "".to_string(),
        _=> vec[0].to_string()
    };
}

fn extract_team_code_from_a_tag(a_tag_selector: &Selector, parent_element: ElementRef) -> String {
    parent_element.select(&a_tag_selector).next().unwrap().value().attr("href").unwrap().split("/").collect::<Vec<&str>>()[5].to_string()
}

fn get_teams(html: String) -> Vec<Team> {
    let fragment = Html::parse_fragment(&html);
    let team_links_selector = Selector::parse("section.TeamLinks").unwrap();
    let team_links = fragment.select(&team_links_selector);
    let mut vec = Vec::new();

    for team_link in team_links {
        let a_selector = Selector::parse("div.pl3 a.AnchorLink").unwrap();
        let a = team_link.select(&a_selector).next().unwrap();
        let team_page_link = a.value().attr("href").unwrap().split("/").collect::<Vec<&str>>()[5].to_string();

        let h2_selector = Selector::parse("h2").unwrap();
        let team_name_h2 = a.select(&h2_selector).next().unwrap().text().collect::<Vec<_>>()[0].to_string();
        vec.push(Team {
            id: team_page_link.to_string(),
            full_name: team_name_h2
        });
    }
    return vec;


}
fn get_latest_game_box(html: &String, home_or_away: HomeOrAway) -> TeamBox {
    let fragment = Html::parse_fragment(&html);
    let css_selector = format!("{}{}{}", ".gamepackage-", home_or_away.to_string(), "-wrap table");
    let table_selector = Selector::parse(&css_selector).unwrap();
    let tr_selector = Selector::parse("tr:not(.highlight)").unwrap();
    let td_selector = Selector::parse("td").unwrap();
    let tbodys = fragment.select(&table_selector).next().unwrap();
    let mut player_lines: Vec<Player> = vec![];


    for tr in tbodys.select(&tr_selector) {
        let mut player = PlayerBoxScore {
            starter: false,
            first_initial_and_last_name: "".to_string(),
            player_id: "".to_string(),
            position: "".to_string(),
            minutes: "".to_string(),
            field_goals_made: "".to_string(),
            field_goals_attempted: "".to_string(),
            three_point_field_goals_made: "".to_string(),
            three_point_field_goals_attempted: "".to_string(),
            free_throws_made: "".to_string(),
            free_throws_attempted: "".to_string(),
            oreb: "".to_string(),
            dreb: "".to_string(),
            rebounds_total: "".to_string(),
            assists: "".to_string(),
            steals: "".to_string(),
            blocked_shots: "".to_string(),
            turnovers: "".to_string(),
            pf: "".to_string(),
            plus_minus: "".to_string(),
            points: "".to_string(),
            dnp: "".to_string(),
            headshots: HashMap::new()
        };
        let mut player_id = String::new();
        let mut valid_row = false;
        let mut player_count = 0;
        for td in tr.select(&td_selector) {
            let name = td.value().attr("class").unwrap();
            let td_contents = td.text().collect::<Vec<_>>();
            let first_value = td_contents[0].to_string();
            match name.as_ref() {
                "name" => {

                    let a_selector = Selector::parse("a").unwrap();
                    let mut a_tag = td.select(&a_selector);
                    valid_row = true;
                    player.first_initial_and_last_name = first_value;
                    player_id.push_str(&a_tag.next().unwrap().value().attr("href").unwrap().split("/").collect::<Vec<&str>>()[7].to_string());
                    player.player_id = player_id.clone();
                    let mut headshots: HashMap<String, String> = HashMap::new();
                    headshots.insert("w192xh192".to_string(),format!("https://a.espncdn.com/combiner/i?img=/i/headshots/nba/players/full/{}.png&w=350&h=254", player_id));
                    player.headshots = headshots;
                    if td_contents.len() >= 3 {
                        player.position = td_contents[2].to_string();
                    }
                }
                "min" => player.minutes = first_value,
                "fg" =>  {
                    player.field_goals_made = first_value.split("-").collect::<Vec<&str>>()[0].to_string();
                    player.field_goals_attempted = first_value.split("-").collect::<Vec<&str>>()[1].to_string();
                }
                "3pt" => {
                    player.three_point_field_goals_made = first_value.split("-").collect::<Vec<&str>>()[0].to_string();
                    player.three_point_field_goals_attempted = first_value.split("-").collect::<Vec<&str>>()[1].to_string();
                }
                "ft" => {
                    player.free_throws_made = first_value.split("-").collect::<Vec<&str>>()[0].to_string();
                    player.free_throws_attempted = first_value.split("-").collect::<Vec<&str>>()[1].to_string();
                },
                "oreb" => player.oreb = first_value,
                "dreb" => player.dreb = first_value,
                "reb" => player.rebounds_total = first_value,
                "ast" => player.assists = first_value,
                "stl" => player.steals = first_value,
                "blk" => player.blocked_shots = first_value,
                "to" => player.turnovers = first_value,
                "pf" => player.pf = first_value,
                "plusminus" => player.plus_minus = first_value,
                "pts" => player.points = first_value,
                "dnp" => player.dnp = first_value,
                _ => ()
            }
        }
        if valid_row {
            player_count += 1;
            player.starter = player_count <= 5;
            player_lines.push(Player {
                player: player,
                id: player_id,
                alignment: home_or_away.to_string()
            });

        }

    }
    let mut manager = HashMap::new();
    manager.insert("image".to_string(), "https://i.imgur.com/QkbchIz.jpg".to_string());
    manager.insert("name".to_string(), "Nick Nurse".to_string());
    return TeamBox {
        overview: get_game_header(&html),
        player_records: player_lines,
        manager: manager
    };
}

fn get_orientation(html: &String, team_code: &str) -> HomeOrAway {
    let fragment = Html::parse_fragment(&html);
    let selector = Selector::parse(".team-info-wrapper a.team-name").unwrap();
    let first_a_tag = fragment.select(&selector).next().unwrap();
    let away_team = Some(first_a_tag.value().attr("href").unwrap().split("/").collect::<Vec<&str>>()[5].to_string());
    let result = *team_code == away_team.unwrap();
    match result {
        true => HomeOrAway::away,
        _ => HomeOrAway::home
    }

}

#[test]
fn get_teams_test() {
    let contents = fs::read_to_string("./test-data/teams-page.html");
    let teams = get_teams(contents.unwrap());
    assert_eq!(teams[0].id, String::from("bos"));
    assert_eq!(teams[0].full_name, String::from("Boston Celtics"));
    assert_eq!(teams[29].id, String::from("sa"));
    assert_eq!(teams[29].full_name, String::from("San Antonio Spurs"));

}


#[test]
fn get_latest_game_id_game_over_test() {
    let contents = fs::read_to_string("./test-data/team-page-game-over.html");
    assert_eq!(get_latest_game_id(contents.unwrap()), String::from("401365914"));
}

#[test]
fn get_latest_game_id_live_game_test() {
    let contents = fs::read_to_string("./test-data/team-page-live-game.html");
    assert_eq!(get_latest_game_id(contents.unwrap()), String::from("401365915"));
}

#[test]
fn get_latest_game_away_box_test() {
    let contents = fs::read_to_string("./test-data/raptors-away-box.html");
    let team_box = get_latest_game_box(&contents.unwrap(), HomeOrAway::away);
    assert_eq!(team_box.player_records[0].player.first_initial_and_last_name, "P. Siakam");
    assert_eq!(team_box.player_records[0].player.player_id, "3149673");
    assert_eq!(team_box.player_records[0].player.free_throws_made, "5");
    assert_eq!(team_box.player_records[0].player.free_throws_attempted, "7");
    assert_eq!(team_box.player_records[9].player.first_initial_and_last_name, "J. Harris");
    assert_eq!(team_box.player_records[11].player.first_initial_and_last_name, "A. Baynes");
    assert_eq!(team_box.player_records[11].player.dnp, "DNP-COACH'S DECISION");
    assert_eq!(team_box.overview.event.away_team.medium_name, "Raptors");
    assert_eq!(team_box.overview.event.home_team.medium_name, "Lakers");
    assert_eq!(team_box.overview.event.home_team.id, "lal");
    assert_eq!(team_box.overview.event.away_team.id, "tor");
    assert_eq!(team_box.overview.event.away_team.logos.w72xh72, "https://a.espncdn.com/combiner/i?img=/i/teamlogos/nba/500/tor.png&h=100&w=100");
    assert_eq!(team_box.overview.event.home_team.logos.w72xh72, "https://a.espncdn.com/combiner/i?img=/i/teamlogos/nba/500/lal.png&h=100&w=100");
    assert_eq!(team_box.overview.score.away.score, "121");
    assert_eq!(team_box.overview.score.home.score, "114");
    assert_eq!(team_box.overview.share_url, "https://www.espn.com/nba/boxscore/_/gameId/401307777");
}

#[test]
fn get_latest_game_home_box_test() {
    let contents = fs::read_to_string("./test-data/raptors-home-box.html");
    let team_box = get_latest_game_box(&contents.unwrap(), HomeOrAway::home);
    assert_eq!(team_box.player_records[0].player.first_initial_and_last_name, "P. Siakam");
    assert_eq!(team_box.player_records[0].player.starter, true);
    assert_eq!(team_box.player_records[0].player.player_id, "3149673");
    assert_eq!(team_box.player_records[6].player.first_initial_and_last_name, "Y. Watanabe");
    assert_eq!(team_box.player_records[6].player.starter, false);
    assert_eq!(team_box.player_records[11].player.first_initial_and_last_name, "A. Baynes");
    assert_eq!(team_box.player_records[6].player.starter, false);
    assert_eq!(team_box.player_records[11].player.dnp, "DNP-COACH'S DECISION");
    assert_eq!(team_box.overview.event.away_team.medium_name, "Nets");
    assert_eq!(team_box.overview.event.home_team.medium_name, "Raptors");
    assert_eq!(team_box.overview.event.home_team.id, "tor");
    assert_eq!(team_box.overview.event.away_team.id, "bkn");
    assert_eq!(team_box.overview.event.away_team.logos.w72xh72, "https://a.espncdn.com/combiner/i?img=/i/teamlogos/nba/500/bkn.png&h=100&w=100");
    assert_eq!(team_box.overview.event.home_team.logos.w72xh72, "https://a.espncdn.com/combiner/i?img=/i/teamlogos/nba/500/tor.png&h=100&w=100");
    assert_eq!(team_box.overview.score.away.score, "116");
    assert_eq!(team_box.overview.score.home.score, "103");
    assert_eq!(team_box.overview.share_url, "https://www.espn.com/nba/boxscore/_/gameId/401307733");
}

#[test]
fn get_orientation_home_test() {
    let contents = fs::read_to_string("./test-data/raptors-home-box.html");
    assert_eq!(get_orientation(&contents.unwrap(), "tor").to_string(), HomeOrAway::home.to_string());
}

#[test]
fn get_orientation_away_test() {
    let contents = fs::read_to_string("./test-data/raptors-away-box.html");
    assert_eq!(get_orientation(&contents.unwrap(), "tor").to_string(), HomeOrAway::away.to_string());
}

#[test]
fn get_upcoming_opponent_team_code_test() {
    let contents = fs::read_to_string("./test-data/raptors-team-page-upcoming-opponent.html");
    assert_eq!(get_upcoming_opponent_team_code(contents.unwrap()).to_string(), "no".to_string());
}


fn probable_lineups(players: &Vec<Player>) -> HashMap<String, Vec<Player>> {
    let mut by_position = HashMap::new();
    for  e in players.iter() {
        let position: String;
        match e.player.position.as_ref() {
            "F" => { position = "PF".to_string() }
            "G" => { position = "SG".to_string() }
            _ => { position = e.player.position.to_string()}
        }
        let px = position.to_string();
        by_position.entry(position).or_insert(Vec::new()).push(
            blank_player(e.player.first_initial_and_last_name.to_string(), px, e.player.starter)
        )
    }
    return by_position;
}

fn injuries(html: String) -> Vec<TeamInjuryReport> {
    let fragment = Html::parse_fragment(&html);
    // let description = get_first_text_value(row, &Selector::parse("injuries__teamName").unwrap());
    let mut team_injury_reports = Vec::new();
    for div in fragment.select(&Selector::parse("div.Table__league-injuries").unwrap()) {
        let mut team_injury_report = TeamInjuryReport {
            team_code: "".to_string(),
            team_name: "".to_string(),
            injuries: vec![]
        };
        team_injury_report.team_name = get_first_text_value(div, &Selector::parse(".injuries__teamName").unwrap());
        for row in div.select(&Selector::parse("tbody tr.Table__TR").unwrap()) {
            team_injury_report.injuries.push(PlayerInjury {
                name: get_first_text_value(row, &Selector::parse("td.col-name a").unwrap()),
                date: get_first_text_value(row, &Selector::parse("td.col-date").unwrap()),
                position: get_first_text_value(row, &Selector::parse("td.col-pos").unwrap()),
                status: get_first_text_value(row, &Selector::parse("td.col-stat span").unwrap()),
                description: get_first_text_value(row, &Selector::parse("td.col-desc").unwrap())
            });
        };
        team_injury_reports.push(team_injury_report);
    }
    return team_injury_reports;
}

#[test]
fn injuries_test() {
    let contents = fs::read_to_string("./test-data/injuries.html");
    let team_injury_reports = injuries(contents.unwrap());
    assert_eq!(team_injury_reports[0].team_name, "Atlanta Hawks");
    assert_eq!(team_injury_reports[1].team_name, "Boston Celtics");
    assert_eq!(team_injury_reports[1].injuries[0].name, "Brodric Thomas");
    assert_eq!(team_injury_reports[1].injuries[0].position, "G");
    assert_eq!(team_injury_reports[1].injuries[0].date, "Jan 9");
    assert_eq!(team_injury_reports[1].injuries[0].description, "Thomas (back) is listed as probable for Monday's game against the Pacers.");
    assert_eq!(team_injury_reports[4].injuries.len(), 4);
    assert_eq!(team_injury_reports.len(), 30);
    assert_eq!(team_injury_reports[29].team_name, "Washington Wizards");
}

#[test]
fn probable_lineups_test() {
    let players = vec![
        blank_player("PF1".to_string(), "PF".to_string(), true),
        blank_player("SF1".to_string(), "SF".to_string(), true),
        blank_player("C1".to_string(), "C".to_string(), true),
        blank_player("PG1".to_string(), "PG".to_string(), true),
        blank_player("SG1".to_string(), "SG".to_string(), true),
        blank_player("PF2".to_string(), "PF".to_string(), false),
        blank_player("SF2".to_string(), "SF".to_string(), false),
        blank_player("C2".to_string(), "C".to_string(), false),
        blank_player("PG2".to_string(), "PG".to_string(), false),
        blank_player("SG2".to_string(), "SG".to_string(), false),
        blank_player("PF3".to_string(), "PF".to_string(), false),
        blank_player("SF3".to_string(), "SF".to_string(), false),
        blank_player("PF4".to_string(), "F".to_string(), false),
        blank_player("PG3".to_string(), "PG".to_string(), false),
        blank_player("SG3".to_string(), "SG".to_string(), false),
        blank_player("SG4".to_string(), "SG".to_string(), false),
        blank_player("SG5".to_string(), "G".to_string(), false)
    ];
    let lineup = probable_lineups(&players);



    // starters must be on each
    assert_eq!(lineup.get("PF").unwrap()[0].player.first_initial_and_last_name, "PF1");
    assert_eq!(lineup.get("PF").unwrap()[1].player.first_initial_and_last_name, "PF2");
    assert_eq!(lineup.get("PF").unwrap()[2].player.first_initial_and_last_name, "PF3");
    assert_eq!(lineup.get("PF").unwrap()[3].player.first_initial_and_last_name, "PF4");
    assert_eq!(lineup.get("PF").unwrap().len(), 4);


    assert_eq!(lineup.get("SF").unwrap()[0].player.first_initial_and_last_name, "SF1");
    assert_eq!(lineup.get("SF").unwrap()[1].player.first_initial_and_last_name, "SF2");
    assert_eq!(lineup.get("SF").unwrap()[2].player.first_initial_and_last_name, "SF3");
    assert_eq!(lineup.get("SF").unwrap().len(), 3);

    assert_eq!(lineup.get("PG").unwrap()[0].player.first_initial_and_last_name, "PG1");
    assert_eq!(lineup.get("PG").unwrap()[1].player.first_initial_and_last_name, "PG2");
    assert_eq!(lineup.get("PG").unwrap()[2].player.first_initial_and_last_name, "PG3");
    assert_eq!(lineup.get("PG").unwrap().len(), 3);

    assert_eq!(lineup.get("C").unwrap()[0].player.first_initial_and_last_name, "C1");
    assert_eq!(lineup.get("C").unwrap()[1].player.first_initial_and_last_name, "C2");
    assert_eq!(lineup.get("C").unwrap().len(), 2);

    assert_eq!(lineup.get("SG").unwrap()[0].player.first_initial_and_last_name, "SG1");
    assert_eq!(lineup.get("SG").unwrap()[1].player.first_initial_and_last_name, "SG2");
    assert_eq!(lineup.get("SG").unwrap()[2].player.first_initial_and_last_name, "SG3");
    assert_eq!(lineup.get("SG").unwrap()[3].player.first_initial_and_last_name, "SG4");
    assert_eq!(lineup.get("SG").unwrap()[4].player.first_initial_and_last_name, "SG5");
    assert_eq!(lineup.get("SG").unwrap().len(), 5);
}

fn blank_player(name: String, position: String, starter: bool) -> Player {
    return Player {
        id: name.to_string(),
        alignment: "".to_string(),
        player: PlayerBoxScore {
            starter,
            first_initial_and_last_name: name.to_string(),
            player_id: "".to_string(),
            position: position.to_string(),
            minutes: "".to_string(),
            field_goals_made: "".to_string(),
            field_goals_attempted: "".to_string(),
            three_point_field_goals_made: "".to_string(),
            three_point_field_goals_attempted: "".to_string(),
            free_throws_made: "".to_string(),
            free_throws_attempted: "".to_string(),
            oreb: "".to_string(),
            dreb: "".to_string(),
            rebounds_total: "".to_string(),
            assists: "".to_string(),
            steals: "".to_string(),
            blocked_shots: "".to_string(),
            turnovers: "".to_string(),
            pf: "".to_string(),
            plus_minus: "".to_string(),
            points: "".to_string(),
            dnp: "".to_string(),
            headshots: HashMap::new()
        }
    };
}
