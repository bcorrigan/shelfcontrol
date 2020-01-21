use failure::Fail;
use tantivy::directory::error::OpenDirectoryError;
use tantivy::query::QueryParserError;
use tantivy::query::QueryParserError::*;
use tantivy::TantivyError;

//Not runtime errors (so doesn't derive Fail)
//Rather, error caused by client use eg. search for non-existent field
#[derive(Serialize, Debug, Fail)]
pub struct ClientError {
	pub name: String,
	pub msg: String,
}

impl std::fmt::Display for ClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "({}, {})", self.name, self.msg)
	}
}

#[derive(Debug, Fail)]
pub enum StoreError {
	//#[fail(display = "IO error:'{}'", _0)]
	//IOError(#[cause] std::io::Error),
	#[fail(display = "Error with store:'{}'", _0)]
	DbError(String),
	#[fail(display = "Error initialising store:'{}'", _0)]
	InitError(String),
	#[fail(display = "Client Error: {}", _0)]
	ClientError(#[cause] ClientError),
}

impl From<TantivyError> for StoreError {
	fn from(te: TantivyError) -> StoreError {
		StoreError::DbError(te.to_string())
	}
}

impl From<OpenDirectoryError> for StoreError {
	fn from(ode: OpenDirectoryError) -> StoreError {
		StoreError::DbError(ode.to_string())
	}
}

impl From<ClientError> for StoreError {
	fn from(ce: ClientError) -> StoreError {
		StoreError::ClientError(ce)
	}
}

impl From<QueryParserError> for StoreError {
	fn from(qpe: QueryParserError) -> StoreError {
		StoreError::from(ClientError::from(qpe))
	}
}

impl From<QueryParserError> for ClientError {
	fn from(qpe: QueryParserError) -> ClientError {
		match qpe {
			SyntaxError => ClientError {
				name: "Syntax Error".to_string(),
				msg: "There was a syntax error in the search string.".to_string(),
			},
			FieldDoesNotExist(s) => ClientError {
				name: "Field does not Exist".to_string(),
				msg: format!("You searched for a field that does not exist:{}", s),
			},
			ExpectedInt(_) => ClientError {
				name: "Expected Integer".to_string(),
				msg: "Search argument requires an integer.".to_string(),
			},
			ExpectedFloat(_) => ClientError {
				name: "Expected float".to_string(),
				msg: "Search argument requires a float.".to_string(),
			},
			AllButQueryForbidden => ClientError {
				name: "All But query forbidden".to_string(),
				msg: "Queries that only exclude (eg. \"-king\") are forbidden.".to_string(),
			},
			NoDefaultFieldDeclared => ClientError {
				name: "No field declared".to_string(),
				msg: "You must specify a field to query.".to_string(),
			},
			FieldNotIndexed(s) => ClientError {
				name: "Field unknown".to_string(),
				msg: format!("The field you searched for is unknown:{}", s),
			},
			UnknownTokenizer(_, _) => ClientError {
				name: "Unknown Tokenizer".to_string(),
				msg: "The tokenizer for the given field is unknown".to_string(),
			},
			FieldDoesNotHavePositionsIndexed(s) => ClientError {
				name: "Field does not have positions indexed".to_string(),
				msg: format!("Field does not have positions indexed: {}", s),
			},
			RangeMustNotHavePhrase => ClientError {
				name: "Range must not have phrase".to_string(),
				msg: "Range must not have phrase".to_string(),
			},
			DateFormatError(_) => ClientError {
				name: "Date must have correct format".to_string(),
				msg: "Date must have correct format".to_string(),
			},
		}
	}
}
