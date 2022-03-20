use crate::dbs::Level;
use crate::dbs::Options;
use crate::dbs::Runtime;
use crate::dbs::Transaction;
use crate::err::Error;
use crate::sql::algorithm::{algorithm, Algorithm};
use crate::sql::base::{base, Base};
use crate::sql::comment::shouldbespace;
use crate::sql::common::take_u64;
use crate::sql::duration::{duration, Duration};
use crate::sql::error::IResult;
use crate::sql::ident::ident_raw;
use crate::sql::idiom;
use crate::sql::idiom::{Idiom, Idioms};
use crate::sql::kind::{kind, Kind};
use crate::sql::permission::{permissions, Permissions};
use crate::sql::statements::UpdateStatement;
use crate::sql::strand::strand_raw;
use crate::sql::value::{value, values, Value, Values};
use crate::sql::view::{view, View};
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use derive::Store;
use nom::branch::alt;
use nom::bytes::complete::tag_no_case;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::tuple;
use rand::distributions::Alphanumeric;
use rand::rngs::OsRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Store)]
pub enum DefineStatement {
	Namespace(DefineNamespaceStatement),
	Database(DefineDatabaseStatement),
	Login(DefineLoginStatement),
	Token(DefineTokenStatement),
	Scope(DefineScopeStatement),
	Table(DefineTableStatement),
	Event(DefineEventStatement),
	Field(DefineFieldStatement),
	Index(DefineIndexStatement),
}

impl DefineStatement {
	pub async fn compute(
		&self,
		ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		doc: Option<&Value>,
	) -> Result<Value, Error> {
		match self {
			DefineStatement::Namespace(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Database(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Login(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Token(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Scope(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Table(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Event(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Field(ref v) => v.compute(ctx, opt, txn, doc).await,
			DefineStatement::Index(ref v) => v.compute(ctx, opt, txn, doc).await,
		}
	}
}

impl fmt::Display for DefineStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DefineStatement::Namespace(v) => write!(f, "{}", v),
			DefineStatement::Database(v) => write!(f, "{}", v),
			DefineStatement::Login(v) => write!(f, "{}", v),
			DefineStatement::Token(v) => write!(f, "{}", v),
			DefineStatement::Scope(v) => write!(f, "{}", v),
			DefineStatement::Table(v) => write!(f, "{}", v),
			DefineStatement::Event(v) => write!(f, "{}", v),
			DefineStatement::Field(v) => write!(f, "{}", v),
			DefineStatement::Index(v) => write!(f, "{}", v),
		}
	}
}

pub fn define(i: &str) -> IResult<&str, DefineStatement> {
	alt((
		map(namespace, DefineStatement::Namespace),
		map(database, DefineStatement::Database),
		map(login, DefineStatement::Login),
		map(token, DefineStatement::Token),
		map(scope, DefineStatement::Scope),
		map(table, DefineStatement::Table),
		map(event, DefineStatement::Event),
		map(field, DefineStatement::Field),
		map(index, DefineStatement::Index),
	))(i)
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineNamespaceStatement {
	pub name: String,
}

impl DefineNamespaceStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Kv)?;
		// Process the statement
		let key = crate::key::ns::new(&self.name);
		txn.clone().lock().await.set(key, self).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineNamespaceStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE NAMESPACE {}", self.name)
	}
}

fn namespace(i: &str) -> IResult<&str, DefineNamespaceStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = alt((tag_no_case("NS"), tag_no_case("NAMESPACE")))(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	Ok((
		i,
		DefineNamespaceStatement {
			name,
		},
	))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineDatabaseStatement {
	pub name: String,
}

impl DefineDatabaseStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Ns)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::db::new(opt.ns(), &self.name);
		run.add_ns(opt.ns()).await?;
		run.set(key, self).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineDatabaseStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE DATABASE {}", self.name)
	}
}

fn database(i: &str) -> IResult<&str, DefineDatabaseStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = alt((tag_no_case("DB"), tag_no_case("DATABASE")))(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	Ok((
		i,
		DefineDatabaseStatement {
			name,
		},
	))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineLoginStatement {
	pub name: String,
	pub base: Base,
	pub hash: String,
	pub code: String,
}

impl DefineLoginStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		match self.base {
			Base::Ns => {
				// Allowed to run?
				opt.check(Level::Kv)?;
				// Clone transaction
				let run = txn.clone();
				// Claim transaction
				let mut run = run.lock().await;
				// Process the statement
				let key = crate::key::nl::new(opt.ns(), &self.name);
				run.add_ns(opt.ns()).await?;
				run.set(key, self).await?;
				// Ok all good
				Ok(Value::None)
			}
			Base::Db => {
				// Allowed to run?
				opt.check(Level::Ns)?;
				// Clone transaction
				let run = txn.clone();
				// Claim transaction
				let mut run = run.lock().await;
				// Process the statement
				let key = crate::key::dl::new(opt.ns(), opt.db(), &self.name);
				run.add_ns(opt.ns()).await?;
				run.add_db(opt.ns(), opt.db()).await?;
				run.set(key, self).await?;
				// Ok all good
				Ok(Value::None)
			}
			_ => unreachable!(),
		}
	}
}

impl fmt::Display for DefineLoginStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE LOGIN {} ON {} PASSHASH {}", self.name, self.base, self.hash)
	}
}

fn login(i: &str) -> IResult<&str, DefineLoginStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("LOGIN")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ON")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, base) = base(i)?;
	let (i, opts) = login_opts(i)?;
	Ok((
		i,
		DefineLoginStatement {
			name,
			base,
			code: rand::thread_rng()
				.sample_iter(&Alphanumeric)
				.take(128)
				.map(char::from)
				.collect::<String>(),
			hash: match opts {
				DefineLoginOption::Passhash(v) => v,
				DefineLoginOption::Password(v) => Argon2::default()
					.hash_password(v.as_ref(), SaltString::generate(&mut OsRng).as_ref())
					.unwrap()
					.to_string(),
			},
		},
	))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefineLoginOption {
	Password(String),
	Passhash(String),
}

fn login_opts(i: &str) -> IResult<&str, DefineLoginOption> {
	alt((login_pass, login_hash))(i)
}

fn login_pass(i: &str) -> IResult<&str, DefineLoginOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("PASSWORD")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = strand_raw(i)?;
	Ok((i, DefineLoginOption::Password(v)))
}

fn login_hash(i: &str) -> IResult<&str, DefineLoginOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("PASSHASH")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = strand_raw(i)?;
	Ok((i, DefineLoginOption::Passhash(v)))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineTokenStatement {
	pub name: String,
	pub base: Base,
	pub kind: Algorithm,
	pub code: String,
}

impl DefineTokenStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		match self.base {
			Base::Ns => {
				// Allowed to run?
				opt.check(Level::Kv)?;
				// Clone transaction
				let run = txn.clone();
				// Claim transaction
				let mut run = run.lock().await;
				// Process the statement
				let key = crate::key::nt::new(opt.ns(), &self.name);
				run.add_ns(opt.ns()).await?;
				run.set(key, self).await?;
				// Ok all good
				Ok(Value::None)
			}
			Base::Db => {
				// Allowed to run?
				opt.check(Level::Ns)?;
				// Clone transaction
				let run = txn.clone();
				// Claim transaction
				let mut run = run.lock().await;
				// Process the statement
				let key = crate::key::dt::new(opt.ns(), opt.db(), &self.name);
				run.add_ns(opt.ns()).await?;
				run.add_db(opt.ns(), opt.db()).await?;
				run.set(key, self).await?;
				// Ok all good
				Ok(Value::None)
			}
			_ => unreachable!(),
		}
	}
}

impl fmt::Display for DefineTokenStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"DEFINE TOKEN {} ON {} TYPE {} VALUE {}",
			self.name, self.base, self.kind, self.code
		)
	}
}

fn token(i: &str) -> IResult<&str, DefineTokenStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("TOKEN")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ON")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, base) = base(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("TYPE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, kind) = algorithm(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("VALUE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, code) = strand_raw(i)?;
	Ok((
		i,
		DefineTokenStatement {
			name,
			base,
			kind,
			code,
		},
	))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineScopeStatement {
	pub name: String,
	pub code: String,
	pub session: Option<Duration>,
	pub signup: Option<Value>,
	pub signin: Option<Value>,
	pub connect: Option<Value>,
}

impl DefineScopeStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Db)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::sc::new(opt.ns(), opt.db(), &self.name);
		run.add_ns(opt.ns()).await?;
		run.add_db(opt.ns(), opt.db()).await?;
		run.set(key, self).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineScopeStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE SCOPE {}", self.name)?;
		if let Some(ref v) = self.session {
			write!(f, " {}", v)?
		}
		if let Some(ref v) = self.signup {
			write!(f, " {}", v)?
		}
		if let Some(ref v) = self.signin {
			write!(f, " {}", v)?
		}
		if let Some(ref v) = self.connect {
			write!(f, " {}", v)?
		}
		Ok(())
	}
}

fn scope(i: &str) -> IResult<&str, DefineScopeStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SCOPE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, opts) = many0(scope_opts)(i)?;
	Ok((
		i,
		DefineScopeStatement {
			name,
			code: rand::thread_rng()
				.sample_iter(&Alphanumeric)
				.take(128)
				.map(char::from)
				.collect::<String>(),
			session: opts.iter().find_map(|x| match x {
				DefineScopeOption::Session(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			signup: opts.iter().find_map(|x| match x {
				DefineScopeOption::Signup(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			signin: opts.iter().find_map(|x| match x {
				DefineScopeOption::Signin(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			connect: opts.iter().find_map(|x| match x {
				DefineScopeOption::Connect(ref v) => Some(v.to_owned()),
				_ => None,
			}),
		},
	))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefineScopeOption {
	Session(Duration),
	Signup(Value),
	Signin(Value),
	Connect(Value),
}

fn scope_opts(i: &str) -> IResult<&str, DefineScopeOption> {
	alt((scope_session, scope_signup, scope_signin, scope_connect))(i)
}

fn scope_session(i: &str) -> IResult<&str, DefineScopeOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SESSION")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = duration(i)?;
	Ok((i, DefineScopeOption::Session(v)))
}

fn scope_signup(i: &str) -> IResult<&str, DefineScopeOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SIGNUP")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = value(i)?;
	Ok((i, DefineScopeOption::Signup(v)))
}

fn scope_signin(i: &str) -> IResult<&str, DefineScopeOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SIGNIN")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = value(i)?;
	Ok((i, DefineScopeOption::Signin(v)))
}

fn scope_connect(i: &str) -> IResult<&str, DefineScopeOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("CONNECT")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = value(i)?;
	Ok((i, DefineScopeOption::Connect(v)))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineTableStatement {
	pub name: String,
	pub drop: bool,
	pub full: bool,
	pub view: Option<View>,
	pub permissions: Permissions,
}

impl DefineTableStatement {
	pub async fn compute(
		&self,
		ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Db)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::tb::new(opt.ns(), opt.db(), &self.name);
		run.add_ns(opt.ns()).await?;
		run.add_db(opt.ns(), opt.db()).await?;
		run.set(key, self).await?;
		// Check if table is a view
		if let Some(view) = &self.view {
			// Remove the table data
			let key = crate::key::table::new(opt.ns(), opt.db(), &self.name);
			run.delp(key, u32::MAX).await?;
			// Process each foreign table
			for v in view.what.0.iter() {
				// Save the view config
				let key = crate::key::ft::new(opt.ns(), opt.db(), &v.name, &self.name);
				run.set(key, self).await?;
			}
			// Release the transaction
			drop(run);
			// Process each foreign table
			for v in view.what.0.iter() {
				// Process the view data
				let stm = UpdateStatement {
					what: Values(vec![Value::Table(v.clone())]),
					..UpdateStatement::default()
				};
				Arc::new(stm).compute(ctx, opt, txn, doc).await?;
			}
		}
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineTableStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE TABLE {}", self.name)?;
		if self.drop {
			write!(f, " DROP")?
		}
		if self.full {
			write!(f, " SCHEMAFULL")?
		}
		if !self.full {
			write!(f, " SCHEMALESS")?
		}
		if let Some(ref v) = self.view {
			write!(f, " {}", v)?
		}
		write!(f, "{}", self.permissions)?;
		Ok(())
	}
}

fn table(i: &str) -> IResult<&str, DefineTableStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("TABLE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, opts) = many0(table_opts)(i)?;
	Ok((
		i,
		DefineTableStatement {
			name,
			drop: opts
				.iter()
				.find_map(|x| match x {
					DefineTableOption::Drop => Some(true),
					_ => None,
				})
				.unwrap_or_default(),
			full: opts
				.iter()
				.find_map(|x| match x {
					DefineTableOption::Schemafull => Some(true),
					DefineTableOption::Schemaless => Some(false),
					_ => None,
				})
				.unwrap_or_default(),
			view: opts.iter().find_map(|x| match x {
				DefineTableOption::View(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			permissions: opts
				.iter()
				.find_map(|x| match x {
					DefineTableOption::Permissions(ref v) => Some(v.to_owned()),
					_ => None,
				})
				.unwrap_or_default(),
		},
	))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefineTableOption {
	Drop,
	View(View),
	Schemaless,
	Schemafull,
	Permissions(Permissions),
}

fn table_opts(i: &str) -> IResult<&str, DefineTableOption> {
	alt((table_drop, table_view, table_schemaless, table_schemafull, table_permissions))(i)
}

fn table_drop(i: &str) -> IResult<&str, DefineTableOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("DROP")(i)?;
	Ok((i, DefineTableOption::Drop))
}

fn table_view(i: &str) -> IResult<&str, DefineTableOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, v) = view(i)?;
	Ok((i, DefineTableOption::View(v)))
}

fn table_schemaless(i: &str) -> IResult<&str, DefineTableOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SCHEMALESS")(i)?;
	Ok((i, DefineTableOption::Schemaless))
}

fn table_schemafull(i: &str) -> IResult<&str, DefineTableOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("SCHEMAFULL")(i)?;
	Ok((i, DefineTableOption::Schemafull))
}

fn table_permissions(i: &str) -> IResult<&str, DefineTableOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, v) = permissions(i)?;
	Ok((i, DefineTableOption::Permissions(v)))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineEventStatement {
	pub name: String,
	pub what: String,
	pub when: Value,
	pub then: Values,
}

impl DefineEventStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Db)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::ev::new(opt.ns(), opt.db(), &self.what, &self.name);
		run.add_ns(opt.ns()).await?;
		run.add_db(opt.ns(), opt.db()).await?;
		run.add_tb(opt.ns(), opt.db(), &self.what).await?;
		run.set(key, self).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineEventStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"DEFINE EVENT {} ON {} WHEN {} THEN {}",
			self.name, self.what, self.when, self.then
		)
	}
}

fn event(i: &str) -> IResult<&str, DefineEventStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("EVENT")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ON")(i)?;
	let (i, _) = opt(tuple((shouldbespace, tag_no_case("TABLE"))))(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, what) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("WHEN")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, when) = value(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("THEN")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, then) = values(i)?;
	Ok((
		i,
		DefineEventStatement {
			name,
			what,
			when,
			then,
		},
	))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineFieldStatement {
	pub name: Idiom,
	pub what: String,
	pub kind: Option<Kind>,
	pub value: Option<Value>,
	pub assert: Option<Value>,
	pub priority: Option<u64>,
	pub permissions: Permissions,
}

impl DefineFieldStatement {
	pub async fn compute(
		&self,
		_ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		_doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Db)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::fd::new(opt.ns(), opt.db(), &self.what, &self.name.to_string());
		run.add_ns(opt.ns()).await?;
		run.add_db(opt.ns(), opt.db()).await?;
		run.add_tb(opt.ns(), opt.db(), &self.what).await?;
		run.set(key, self).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineFieldStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE FIELD {} ON {}", self.name, self.what)?;
		if let Some(ref v) = self.kind {
			write!(f, " TYPE {}", v)?
		}
		if let Some(ref v) = self.value {
			write!(f, " VALUE {}", v)?
		}
		if let Some(ref v) = self.assert {
			write!(f, " ASSERT {}", v)?
		}
		write!(f, "{}", self.permissions)?;
		Ok(())
	}
}

fn field(i: &str) -> IResult<&str, DefineFieldStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("FIELD")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = idiom::local(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ON")(i)?;
	let (i, _) = opt(tuple((shouldbespace, tag_no_case("TABLE"))))(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, what) = ident_raw(i)?;
	let (i, opts) = many0(field_opts)(i)?;
	Ok((
		i,
		DefineFieldStatement {
			name,
			what,
			kind: opts.iter().find_map(|x| match x {
				DefineFieldOption::Kind(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			value: opts.iter().find_map(|x| match x {
				DefineFieldOption::Value(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			assert: opts.iter().find_map(|x| match x {
				DefineFieldOption::Assert(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			priority: opts.iter().find_map(|x| match x {
				DefineFieldOption::Priority(ref v) => Some(v.to_owned()),
				_ => None,
			}),
			permissions: opts
				.iter()
				.find_map(|x| match x {
					DefineFieldOption::Permissions(ref v) => Some(v.to_owned()),
					_ => None,
				})
				.unwrap_or_default(),
		},
	))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefineFieldOption {
	Kind(Kind),
	Value(Value),
	Assert(Value),
	Priority(u64),
	Permissions(Permissions),
}

fn field_opts(i: &str) -> IResult<&str, DefineFieldOption> {
	alt((field_kind, field_value, field_assert, field_priority, field_permissions))(i)
}

fn field_kind(i: &str) -> IResult<&str, DefineFieldOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("TYPE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = kind(i)?;
	Ok((i, DefineFieldOption::Kind(v)))
}

fn field_value(i: &str) -> IResult<&str, DefineFieldOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("VALUE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = value(i)?;
	Ok((i, DefineFieldOption::Value(v)))
}

fn field_assert(i: &str) -> IResult<&str, DefineFieldOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ASSERT")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = value(i)?;
	Ok((i, DefineFieldOption::Assert(v)))
}

fn field_priority(i: &str) -> IResult<&str, DefineFieldOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("PRIORITY")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, v) = take_u64(i)?;
	Ok((i, DefineFieldOption::Priority(v)))
}

fn field_permissions(i: &str) -> IResult<&str, DefineFieldOption> {
	let (i, _) = shouldbespace(i)?;
	let (i, v) = permissions(i)?;
	Ok((i, DefineFieldOption::Permissions(v)))
}

// --------------------------------------------------
// --------------------------------------------------
// --------------------------------------------------

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, Store)]
pub struct DefineIndexStatement {
	pub name: String,
	pub what: String,
	pub cols: Idioms,
	pub uniq: bool,
}

impl DefineIndexStatement {
	pub async fn compute(
		&self,
		ctx: &Runtime,
		opt: &Options,
		txn: &Transaction,
		doc: Option<&Value>,
	) -> Result<Value, Error> {
		// Allowed to run?
		opt.check(Level::Db)?;
		// Clone transaction
		let run = txn.clone();
		// Claim transaction
		let mut run = run.lock().await;
		// Process the statement
		let key = crate::key::ix::new(opt.ns(), opt.db(), &self.what, &self.name);
		run.add_ns(opt.ns()).await?;
		run.add_db(opt.ns(), opt.db()).await?;
		run.add_tb(opt.ns(), opt.db(), &self.what).await?;
		run.set(key, self).await?;
		// Remove the index data
		let key = crate::key::index::new(opt.ns(), opt.db(), &self.what, &self.name, Value::None);
		run.delp(key, u32::MAX).await?;
		// Release the transaction
		drop(run);
		// Update the index data
		let stm = UpdateStatement {
			what: Values(vec![Value::Table(self.what.clone().into())]),
			..UpdateStatement::default()
		};
		Arc::new(stm).compute(ctx, opt, txn, doc).await?;
		// Ok all good
		Ok(Value::None)
	}
}

impl fmt::Display for DefineIndexStatement {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "DEFINE INDEX {} ON {} COLUMNS {}", self.name, self.what, self.cols)?;
		if self.uniq {
			write!(f, " UNIQUE")?
		}
		Ok(())
	}
}

fn index(i: &str) -> IResult<&str, DefineIndexStatement> {
	let (i, _) = tag_no_case("DEFINE")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("INDEX")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, name) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("ON")(i)?;
	let (i, _) = opt(tuple((shouldbespace, tag_no_case("TABLE"))))(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, what) = ident_raw(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, _) = tag_no_case("COLUMNS")(i)?;
	let (i, _) = shouldbespace(i)?;
	let (i, cols) = idiom::locals(i)?;
	let (i, uniq) = opt(|i| {
		shouldbespace(i)?;
		tag_no_case("UNIQUE")(i)?;
		Ok((i, true))
	})(i)?;
	Ok((
		i,
		DefineIndexStatement {
			name,
			what,
			cols,
			uniq: uniq.is_some(),
		},
	))
}