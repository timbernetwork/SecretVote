use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, StdError,
    StdResult, Timestamp, Uint256,
};
use std::cmp::max;

use crate::errors::CustomContractError;
use crate::msg::{
    CountResponse, ExecuteMsg, InstantiateMsg, ProposalResponse, QueryMsg, RicherResponse,
    WinnerResponse,
};
use crate::state::{
    config, config_read, ContractState, Millionaire, Proposal, ProposalVoter, State,
    COUNT_STORE, PROPOSALS_STORE
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let mut state = State::default();
    state.count_static = Uint256::from(1337u32);
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, CustomContractError> {
    match msg {
        ExecuteMsg::Increment {} => try_increment(deps),
        ExecuteMsg::SubmitNetWorth { name, worth } => try_submit_net_worth(deps, name, worth),
        ExecuteMsg::Reset {} => try_reset(deps),
        ExecuteMsg::SubmitProposal {
            id,
            choice_count,
            start_time,
            end_time,
        } => try_add_proposal(deps, id, choice_count, start_time, end_time),
        ExecuteMsg::RegisterVoter {
            proposal_id,
            eth_addr,
            scrt_addr,
            power,
        } => try_register_voter(deps, proposal_id, eth_addr, scrt_addr, power),
        ExecuteMsg::CastVote {
            proposal_id,
            eth_addr,
            scrt_addr,
            choice,
        } => try_cast_vote(deps, proposal_id, eth_addr, scrt_addr, choice),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::CurrentProposal {} => to_binary(&query_current_proposal(deps)?),
        QueryMsg::VoterCount {} => to_binary(&query_voter_count(deps)?),
        QueryMsg::WhoWon { proposal_id } => to_binary(&query_count_vote_results(deps, &proposal_id)?),
        QueryMsg::WhoIsRicher {} => to_binary(&query_who_is_richer(deps)?),
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::GetCountStatic {} => to_binary(&query_count_static(deps)?),
    }
}

pub fn try_register_voter(
    deps: DepsMut,
    proposal_id: String,
    eth_addr: String,
    scrt_addr: String,
    power: Uint256,
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;
    if state.voter1.scrt_addr == "" {
        state.voter1 = ProposalVoter::register(proposal_id, eth_addr, scrt_addr, power);
    } else {
        // XXX
        state.voter2 = ProposalVoter::register(proposal_id, eth_addr, scrt_addr, power);
    }
    config(deps.storage).save(&state)?;
    println!("try register voter state: {:?}", state);

    Ok(Response::new())
}

pub fn try_cast_vote(
    deps: DepsMut,
    proposal_id: String,
    eth_addr: String,
    scrt_addr: String,
    choice: u8,
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;
    println!(
        "proposal {:?} should be state-like {:?}",
        proposal_id, state.prop.id
    );
    println!("should look up by eth addr {:?}", eth_addr);
    let power: Uint256;
    if state.voter1.scrt_addr == scrt_addr {
        power = state.voter1.power;
    } else if state.voter2.scrt_addr == scrt_addr {
        power = state.voter2.power;
    } else {
        // XXX
        power = state.voter3.power;
    }
    // TODO don't let him vote twice
    match choice {
        0 => state.counter1 += power,
        1 => state.counter2 += power,
        2 => state.counter3 += power,
        _ => state.counter4 += power,
    }
    config(deps.storage).save(&state)?;
    println!("try cast voter state: {:?}", state);

    Ok(Response::new())
}

pub fn try_add_proposal(
    deps: DepsMut,
    id: String,
    choice_count: u8,
    start_time: Timestamp,
    end_time: Timestamp,
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;
    // TODO clear existing counters and voters
    // state.voter1 = ProposalVoter::default();
    state.prop = Proposal::new(id.clone(), choice_count, start_time, end_time);
    // XXX state.proposals.push(Proposal::new(id, choice_count, start_time, end_time));
    config(deps.storage).save(&state)?;
    println!("try add proposal state: {:?}", state);

    PROPOSALS_STORE.push(deps.storage, &state.prop.clone());
    // Test AppendStore
    COUNT_STORE.push(deps.storage, &2)?;
    COUNT_STORE.push(deps.storage, &3)?;
    COUNT_STORE.push(deps.storage, &5)?;
    COUNT_STORE.push(deps.storage, &8)?;
    COUNT_STORE.push(deps.storage, &11)?;

    Ok(Response::new())
}

pub fn try_increment(deps: DepsMut) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;
    state.count += Uint256::from(1u32); 
   // state.count_static = 666;
    config(deps.storage).save(&state)?;
    Ok(Response::new())
}

pub fn try_submit_net_worth(
    deps: DepsMut,
    name: String,
    worth: u64,
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;

    match state.state {
        ContractState::Init => {
            state.player1 = Millionaire::new(name, worth);
            state.state = ContractState::Got1;
        }
        ContractState::Got1 => {
            state.player2 = Millionaire::new(name, worth);
            state.state = ContractState::Done;
        }
        ContractState::Done => {
            return Err(CustomContractError::AlreadyAddedBothMillionaires);
        }
    }

    config(deps.storage).save(&state)?;

    Ok(Response::new())
}

pub fn try_reset(deps: DepsMut) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;

    state.state = ContractState::Init;
    config(deps.storage).save(&state)?;

    Ok(Response::new().add_attribute("action", "reset state"))
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    //let state = STATE.load(deps.storage)?;
    let state = config_read(deps.storage).load()?;
    // Load the current contract state
    Ok(CountResponse { count: state.count })
    // Form and return a CountResponse
}
fn query_count_static(deps: Deps) -> StdResult<CountResponse> {
    //let state = STATE.load(deps.storage)?;
    let state = config_read(deps.storage).load()?;
    // Load the current contract state
    Ok(CountResponse {
        count: state.count_static,
    })
    // Form and return a CountResponse
}
fn query_who_is_richer(deps: Deps) -> StdResult<RicherResponse> {
    let state = config_read(deps.storage).load()?;

    if state.state != ContractState::Done {
        return Err(StdError::generic_err(
            "Can't tell who is richer unless we get 2 data points!",
        ));
    }

    if state.player1 == state.player2 {
        let resp = RicherResponse {
            richer: "It's a tie!".to_string(),
        };

        return Ok(resp);
    }

    let richer = max(state.player1, state.player2);

    let resp = RicherResponse {
        // we use .clone() here because ...
        richer: richer.name().clone(),
    };

    Ok(resp)
}

fn query_count_vote_results(deps: Deps, proposal_id: &str) -> StdResult<WinnerResponse> {
    let state = config_read(deps.storage).load()?;
    println!("compare requested {:?} with state {:?}", proposal_id, state.prop.id);
    if state.counter1 > state.counter2 {
        let resp = WinnerResponse {
            choice: 0,
            choice_count: state.counter1,
        };
        return Ok(resp);
    }

    // TODO more than 2 choices
    let resp = WinnerResponse {
        choice: 1,
        choice_count: state.counter2,
    };
    Ok(resp)
}

fn query_current_proposal(
    deps: Deps,
    //proposal_id: &str,
) -> StdResult<ProposalResponse> {
    // COUNT_STORE.push(deps.storage, &1234)?;
    // let fake_choice_count = match COUNT_STORE.get_len(deps.storage) {
    let fake_choice_count = match COUNT_STORE.get_at(deps.storage, 3) {
        Ok(l) => l as u8,
        Err(e) => return Err(e),
    };
    /*
    let state = config_read(deps.storage).load()?;
    let resp = ProposalResponse {
        id: state.prop.id,
        choice_count: fake_choice_count, // state.prop.choice_count,
    };
    println!("resp {:?}", resp);
    */
    let prop_len = PROPOSALS_STORE.get_len(deps.storage)?;
    let prop = PROPOSALS_STORE.get_at(deps.storage, prop_len-1)?;
    /*
    let prop: Proposal = match PROPOSALS_STORE.get_at(deps.storage, prop_len) {
        Ok(res) => res,
        Err(e) => return Err(e),
    };
    */
    let resp = ProposalResponse {
        id: prop.id,
        choice_count: prop.choice_count, // state.prop.choice_count,
    };
    Ok(resp)
}
fn query_voter_count(
    deps: Deps,
    //proposal_id: &str,
) -> StdResult<CountResponse> {
    let state = config_read(deps.storage).load()?;
    let mut cnt: u32 = 0;
    if state.voter1.scrt_addr == "" {
    } else {
        if state.voter2.scrt_addr == "" {
            cnt = 1;
        } else {
            cnt = 2;
        }
    }
    let resp = CountResponse { count: Uint256::from(cnt) };
    println!("resp {:?}", resp);
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockStorage};
    use secret_toolkit::storage::AppendStore;

    // the trait `cosmwasm_std::traits::Storage` is not implemented for `MemoryStorage` 
    // the trait `From<cosmwasm_std::errors::std_error::StdError>` is not implemented for `cosmwasm_std::StdError`
    #[test]
    fn cwspmap_demo() -> StdResult<()> {
        /*
        let mut store = MockStorage::new();
        let data = Data {
            name: "John".to_string(),
            age: 32,
        };
        */

        // load and save with extra key argument
        // the trait `cosmwasm_std::traits::Storage` is not implemented for `MemoryStorage` 
        // let empty = PEOPLE.may_load(&store, "john")?;
    /*
        assert_eq!(None, empty);
        PEOPLE.save(&mut store, "john", &data)?;
        let loaded = PEOPLE.load(&store, "john")?;
        assert_eq!(data, loaded);

        // nothing on another key
        let missing = PEOPLE.may_load(&store, "jack")?;
        assert_eq!(None, missing);

        // update function for new or existing keys
        let birthday = |d: Option<Data>| -> StdResult<Data> {
            match d {
                Some(one) => Ok(Data {
                    name: one.name,
                    age: one.age + 1,
                }),
                None => Ok(Data {
                    name: "Newborn".to_string(),
                    age: 0,
                }),
            }
        };

        let old_john = PEOPLE.update(&mut store, "john", birthday)?;
        assert_eq!(33, old_john.age);
        assert_eq!("John", old_john.name.as_str());

        let new_jack = PEOPLE.update(&mut store, "jack", birthday)?;
        assert_eq!(0, new_jack.age);
        assert_eq!("Newborn", new_jack.name.as_str());

        // update also changes the store
        assert_eq!(old_john, PEOPLE.load(&store, "john")?);
        assert_eq!(new_jack, PEOPLE.load(&store, "jack")?);

        // removing leaves us empty
        PEOPLE.remove(&mut store, "john");
        let empty = PEOPLE.may_load(&store, "john")?;
        assert_eq!(None, empty);

    */
        Ok(())
    }

    #[test]
    fn test_push_pop() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let append_store: AppendStore<i32> = AppendStore::new(b"test");
        /* the trait bound `MemoryStorage: secret_cosmwasm_std::traits::Storage` is not satisfied
            the following other types implement trait `secret_cosmwasm_std::traits::Storage`:
            secret_cosmwasm_std::storage::MemoryStorage
            secret_cosmwasm_storage::prefixed_storage::PrefixedStorage<'a>
            secret_cosmwasm_storage::prefixed_storage::ReadonlyPrefixedStorage<'a>
            required for the cast from `MemoryStorage` to the object type `dyn secret_cosmwasm_std::traits::Storage`
        */
        append_store.push(&mut storage, &1234)?;
        append_store.push(&mut storage, &2143)?;
        append_store.push(&mut storage, &3412)?;
        append_store.push(&mut storage, &4321)?;

        assert_eq!(append_store.pop(&mut storage), Ok(4321));
        assert_eq!(append_store.pop(&mut storage), Ok(3412));
        assert_eq!(append_store.pop(&mut storage), Ok(2143));
        assert_eq!(append_store.pop(&mut storage), Ok(1234));
        assert!(append_store.pop(&mut storage).is_err());
        Ok(())
    }

    #[test]
    fn proper_instantialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let _ = query_who_is_richer(deps.as_ref()).unwrap_err();
    }

    #[test]
    fn cast_vote1() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();


        let proposal = ExecuteMsg::SubmitProposal {
            id: String::from("prop1"),
            choice_count: 4u8,
            start_time: Timestamp::from_nanos(1_000_000_101),
            end_time: Timestamp::from_nanos(1_000_000_202),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();
        assert_eq!(0, res.messages.len());

        let regvo1 = ExecuteMsg::RegisterVoter {
            proposal_id: String::from("prop1"),
            eth_addr: String::from("0xBEEF"),
            scrt_addr: String::from("secretvoter1"),
            power: Uint256::from(100u32),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), regvo1).unwrap();
        assert_eq!(0, res.messages.len());

        let regvo2 = ExecuteMsg::RegisterVoter {
            proposal_id: String::from("prop1"),
            eth_addr: String::from("0xDEAD"),
            scrt_addr: String::from("secretvoter2"),
            power: Uint256::from(250u32),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), regvo2).unwrap();
        assert_eq!(0, res.messages.len());

        let cnt = query_voter_count(deps.as_ref()).unwrap();
        println!("voter cnt {:?}", cnt);

        let cast1 = ExecuteMsg::CastVote {
            proposal_id: String::from("prop1"),
            eth_addr: String::from("0xDEAD"),
            scrt_addr: String::from("secretvoter2"),
            choice: 1,
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), cast1).unwrap();
        assert_eq!(0, res.messages.len());

        let winner = query_count_vote_results(deps.as_ref(), "prop1").unwrap();
        println!("winner  {:?}", winner);

    }

    #[test]
    fn register_voter1() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let regvo1 = ExecuteMsg::RegisterVoter {
            proposal_id: String::from("prop1"),
            eth_addr: String::from("0xBEEF"),
            scrt_addr: String::from("secretvoter1"),
            power: Uint256::from(100u32),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), regvo1).unwrap();
        assert_eq!(0, res.messages.len());

        let regvo2 = ExecuteMsg::RegisterVoter {
            proposal_id: String::from("prop1"),
            eth_addr: String::from("0xDEAD"),
            scrt_addr: String::from("secretvoter2"),
            power: Uint256::from(250u32),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), regvo2).unwrap();
        assert_eq!(0, res.messages.len());

        let cnt = query_voter_count(deps.as_ref()).unwrap();
        println!("voter cnt {:?}", cnt);
    }

    #[test]
    fn add_proposal() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let proposal = ExecuteMsg::SubmitProposal {
            id: String::from("prop1"),
            choice_count: 4u8,
            start_time: Timestamp::from_nanos(1_000_000_101),
            end_time: Timestamp::from_nanos(1_000_000_202),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();
        assert_eq!(0, res.messages.len());

        let proposal = ExecuteMsg::SubmitProposal {
            id: String::from("prop2"),
            choice_count: 3u8,
            start_time: Timestamp::from_nanos(1_000_000_101),
            end_time: Timestamp::from_nanos(1_000_000_202),
        };

        let info = mock_info("creator", &[]);
        let res = execute(deps.as_mut(), mock_env(), info.clone(), proposal).unwrap();
        assert_eq!(0, res.messages.len());

        let _ = query_current_proposal(deps.as_ref()).unwrap();
    }

    #[test]
    fn solve_millionaire() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg_player1 = ExecuteMsg::SubmitNetWorth {
            worth: 1,
            name: "alice".to_string(),
        };
        let msg_player2 = ExecuteMsg::SubmitNetWorth {
            worth: 2,
            name: "bob".to_string(),
        };

        let info = mock_info("creator", &[]);

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_player1).unwrap();
        let _res = execute(deps.as_mut(), mock_env(), info, msg_player2).unwrap();

        // it worked, let's query the state
        let value = query_who_is_richer(deps.as_ref()).unwrap();

        assert_eq!(&value.richer, "bob")
    }

    #[test]
    fn test_reset_state() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg_player1 = ExecuteMsg::SubmitNetWorth {
            worth: 1,
            name: "alice".to_string(),
        };

        let info = mock_info("creator", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_player1).unwrap();

        let reset_msg = ExecuteMsg::Reset {};
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), reset_msg).unwrap();

        let msg_player2 = ExecuteMsg::SubmitNetWorth {
            worth: 2,
            name: "bob".to_string(),
        };
        let msg_player3 = ExecuteMsg::SubmitNetWorth {
            worth: 3,
            name: "carol".to_string(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_player2).unwrap();
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_player3).unwrap();

        // it worked, let's query the state
        let value = query_who_is_richer(deps.as_ref()).unwrap();

        assert_eq!(&value.richer, "carol")
    }
}
