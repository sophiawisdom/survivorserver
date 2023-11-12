use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use warp::Filter;
use sha3::{Digest, Sha3_224};
use base64::{engine::general_purpose, Engine as _};

#[derive(Clone, Serialize)]
struct User {
    id: usize,
    name: String,
    deleted: bool
}

#[derive(Clone, Serialize)]
struct UserVote {
    from_user: usize,
    to_user: usize,
}

#[derive(Clone, Serialize)]
struct Vote {
    id: usize,
    voters: Vec<usize>, // userids
    start: u64,
    end: u64,
    votes: Vec<UserVote>
}

#[derive(Clone)]
struct ServerData {
    users: Vec<User>,
    votes: Vec<Vote>,
}

#[tokio::main]
async fn main() {
    let cors = warp::cors()
    .allow_methods(vec!["GET", "POST"])
    .allow_header("content-type")
    .max_age(10000000)
    .allow_any_origin();

    let server_data = Arc::new(Mutex::new(ServerData{ users: vec![], votes: vec![] }));
    let server_data_votes = server_data.clone();
    let server_data_add_user = server_data.clone();
    let server_data_edit_user = server_data.clone();
    let server_data_add_vote = server_data.clone();
    let server_data_do_vote = server_data.clone();


    let read_users = warp::path!("users").map(move || {
        let server_data_read = server_data.lock().unwrap();
        serde_json::to_string(&server_data_read.users).unwrap()
    });

    let add_user = warp::path!("add_user").and(warp::body::json()).map(move |simple_map: HashMap<String, String>| {
        let mut server_data = server_data_add_user.lock().unwrap();
        let name = simple_map.get("name").unwrap();
        let new_id = server_data.users.len();
        server_data.users.push(User { id: new_id, name: name.clone(), deleted: false });
        new_id.to_string()
    });

    let edit_user = warp::path!("edit_user" / u32).and(warp::body::json()).map(move |user_id: u32, simple_map: HashMap<String, String>| {
        let mut server_data = server_data_edit_user.lock().unwrap();
        let user: &mut User = server_data.users.get_mut(user_id as usize).unwrap();
        let name = simple_map.get("name").unwrap();
        let no_string = "no".to_string();
        let deleted = simple_map.get("deleted").unwrap_or(&no_string);
        user.name = name.clone();
        user.deleted = deleted == "yes";
        "".to_string()
    });

    let create_vote = warp::path!("create_vote").and(warp::body::json()).map(move |simple_map: serde_json::Value| {
        let mut server_data = server_data_add_vote.lock().unwrap();
        let simple_map = simple_map.as_object().unwrap();
        let start = simple_map.get("start").unwrap().as_i64().unwrap();
        let end = simple_map.get("end").unwrap().as_i64().unwrap();
        let voters = simple_map.get("voters").unwrap().as_array().unwrap();
        let mut voters_usize: Vec<usize> = vec![];
        for voter in voters {
            voters_usize.push(voter.as_i64().unwrap() as usize);
        }
        let vote = Vote{start: start as u64, end: end as u64, id: server_data.votes.len(), votes: vec![], voters: voters_usize };
        server_data.votes.push(vote);
        "".to_string()
    });

    let do_vote = warp::path!("vote").and(warp::body::json()).map(move |simple_map: serde_json::Value| {
        let mut server_data = server_data_do_vote.lock().unwrap();
        let simple_map = simple_map.as_object().unwrap();
        let vote_by = simple_map.get("by").unwrap().as_i64().unwrap() as usize;
        let vote_for = simple_map.get("for").unwrap().as_i64().unwrap() as usize;
        let vote_on = simple_map.get("on").unwrap().as_i64().unwrap() as usize;
        let mut vote = server_data.votes.get_mut(vote_on).unwrap();
        match vote.votes.iter_mut().filter(|vote| vote.from_user == vote_by).collect::<Vec<&mut UserVote>>().get_mut(0) {
            Some(val) => {
                val.to_user = vote_for;
            }
            None => {
                vote.votes.push(UserVote{from_user: vote_by, to_user: vote_for});
            }
        }
        "".to_string()
    });

    let read_votes = warp::path!("votes").map(move || {
        let server_data_read = server_data_votes.lock().unwrap();
        serde_json::to_string(&server_data_read.votes).unwrap()
    });


    let options = warp::options().map(warp::reply);

    warp::serve(read_users.or(add_user).or(edit_user).or(create_vote).or(do_vote).or(read_votes).or(options).with(cors))
        .run(([0, 0, 0, 0], 3030))
        .await;
}
