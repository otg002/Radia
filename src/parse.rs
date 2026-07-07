use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Display;
use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    token: TokenKind,
    line: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Root,
    Set,
    Get,
    Let,
    Number(String),
    Identifier(String),
    String(String),
    Character(char),
    Type,
    For,
    Implement,
    Block,
    EndBlock,
    Message,
    EndMessage,
    List,
    EndList,
    Operation,
    EndOperation,
    Semicolon,
    Yield,
    Define,
    Lambda,
    New,
    Unit,
    Self_,
}

fn token_to_kind(token: &str) -> TokenKind {
    match token {
        "set" => TokenKind::Set,
        "get" => TokenKind::Get,
        "let" => TokenKind::Let,
        "impl" => TokenKind::Implement,
        "for" => TokenKind::For,
        "type" => TokenKind::Type,
        "{" => TokenKind::Block,
        "}" => TokenKind::EndBlock,
        "[" => TokenKind::Message,
        "]" => TokenKind::EndMessage,
        "(" => TokenKind::List,
        ")" => TokenKind::EndList,
        "<" => TokenKind::Operation,
        ">" => TokenKind::EndOperation,
        ";" => TokenKind::Semicolon,
        "yield" => TokenKind::Yield,
        "def" => TokenKind::Define,
        "lambda" => TokenKind::Lambda,
        "λ" => TokenKind::Lambda,
        "func" => TokenKind::Lambda,
        "fn" => TokenKind::Lambda,
        "function" => TokenKind::Lambda,
        "\\" => TokenKind::Lambda,
        "new" => TokenKind::New,
        "unit" => TokenKind::Unit,
        "self" => TokenKind::Self_,
        _ => {
            match token.chars().next().expect("Unexpected empty token") {
                '0'..='9' => TokenKind::Number(token.to_string()),
                '\'' => TokenKind::Character(token.chars().collect::<Vec<char>>()[1]),
                '"' => TokenKind::String(token.strip_prefix("\"").unwrap().strip_suffix("\"").unwrap().to_string()),
                _ => TokenKind::Identifier(token.to_string()),
            }
        }
    }
}

fn tokenize(program: String) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut line: usize = 1;
    let mut word: String = String::new();
    let mut comment: bool = false;
    let mut str: bool = false;
    use std::collections::HashSet;
    let mut ops = HashSet::new(); {
        ops.insert('\\');
        ops.insert('λ');
        ops.insert('{');
        ops.insert('}');
        ops.insert(';');
        ops.insert('[');
        ops.insert(']');
        ops.insert('(');
        ops.insert(')');
        ops.insert('<');
        ops.insert('>');
        ops.insert('!');
        ops.insert('?');
    }
    for char in program.chars() {
        if comment {
            if char == '#' {
                comment = false;
            }
        }
        else if str {
            if char == '"' {
                str = false;
                word.push(char);
                tokens.push(Token {token: token_to_kind(word.as_str()), line});
                word.clear();
            }
            else {
                word.push(char);
            }
        }
        else if char == '#' {
            comment = true;
            if !word.is_empty() {
                tokens.push(Token {token: token_to_kind(word.as_str()), line});
            }
            word.clear();
        }
        else if char.is_whitespace() {
            if !word.is_empty() {
                tokens.push(Token {token: token_to_kind(word.as_str()), line});
            }
            word.clear();
        }
        else if ops.contains(&char) {
            if !word.is_empty() {
                tokens.push(Token {token: token_to_kind(word.as_str()), line});
            }
            word.clear();
            tokens.push(Token {token: token_to_kind(char.to_string().as_str()), line});
        }
        else if char == '"' {
            str = true;
            if !word.is_empty() {
                tokens.push(Token {token: token_to_kind(word.as_str()), line});
            }
            word = String::from("\"");
        }
        else {
            word.push(char);
        }
        if char == '\n' {
            line += 1;
        }
    }
    tokens
}

#[derive(Clone, PartialEq)]
pub struct MutTree {
    data: Token,
    children: Vec<Tree>,
    parent: Option<Tree>,
}
type Tree = Rc<RefCell<MutTree>>;

impl MutTree {
    fn new(data: Token, parent: Option<Tree>) -> Tree {
        Rc::new(RefCell::new(MutTree {data, children: Vec::new(), parent}))
    }
    fn root() -> Tree {
        MutTree::new(Token {token: TokenKind::Root, line: 0}, None)
    }
    fn flatten(&self) -> FlatTree<Token> {
        let mut children = Vec::new();
        for child in &self.children {
            children.push(child.borrow().flatten());
        }
        FlatTree {
            data: self.data.clone(),
            children
        }
    }
    fn add_child(&mut self, child: Tree) {
        self.children.push(child);
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FlatTree<T> {
    pub data: T,
    pub children: Vec<FlatTree<T>>,
}
impl<T> FlatTree<T> {
    fn new(data: T, children: Vec<FlatTree<T>>) -> FlatTree<T> {
        FlatTree {data, children}
    }
}
impl<T: Display> Display for FlatTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.data, if self.children.is_empty() {
            String::new()
        } else {
            let mut str: String = String::from(" (\n");
            for child in &self.children {
                str.push_str((format!("\t{}", child).replace("\n", "\n\t") + "\n").as_str());
            }
            str.push_str(")");
            str
        })
    }
}

fn parse_concrete(tokens: Vec<Token>) -> Result<FlatTree<Token>> {
    let mut current: Tree = MutTree::root();
    for token in tokens {
        match token.token {
            TokenKind::Root => {
                return Err(Error::ParseError(token.line, String::from(
                    "Unexpected `Root` token found in program. Likely an internal error."
                )))
            }
            TokenKind::Set
            | TokenKind::Get
            | TokenKind::Let
            | TokenKind::Yield
            | TokenKind::Implement
            | TokenKind::Type
            | TokenKind::Block
            | TokenKind::Message
            | TokenKind::List
            | TokenKind::Operation
            | TokenKind::Define
            | TokenKind::Lambda => {
                let next = MutTree::new(token, Some(current.clone()));
                current.borrow_mut().add_child(next.clone());
                current = next;
            }
            TokenKind::Number(_)
            | TokenKind::Character(_)
            | TokenKind::String(_)
            | TokenKind::Identifier(_)
            | TokenKind::New
            | TokenKind::Self_
            | TokenKind::Unit
            | TokenKind::For => {
                let line = token.line;
                let next = MutTree::new(token, Some(current.clone()));
                current.borrow_mut().add_child(next.clone());
                if let TokenKind::Define = current.clone().borrow().data.token {
                    if current.borrow().children.len() >= 2 {
                        let parent = current.borrow().parent.clone().ok_or(
                            Error::ParseError(line, String::from("Internal error: `def` block escaped root."))
                        )?;
                        current = parent;
                    }
                }
            }
            TokenKind::Semicolon => {
                while current.borrow().data.token != TokenKind::Block {
                    let parent = current.borrow().parent.clone().ok_or(Error::ParseError(token.line, String::from("Unexpected semicolon")))?;
                    current = parent;
                }
            }
            TokenKind::EndBlock => {
                while current.borrow().data.token != TokenKind::Block {
                    let parent = current.borrow().parent.clone().ok_or(Error::ParseError(token.line, String::from("Unmatched `}` or `end` symbol.")))?;
                    current = parent;
                }
                let parent = current.borrow().parent.clone().ok_or(Error::ParseError(token.line, String::from("Unmatched `}` or `end` symbol.")))?;
                current = parent;
                match current.clone().borrow().data.token {
                    TokenKind::Implement | TokenKind::Type => {
                        let parent = current.borrow().parent.clone().ok_or(Error::ParseError(token.line, String::from(
                            "Internal Error: `impl` or `type` block escaped root."
                        )))?;
                        current = parent;
                    }
                    TokenKind::Lambda => {
                        let parent = current.borrow().parent.clone().ok_or(Error::ParseError(token.line, String::from(
                            "Internal Error: `lambda` block escaped root."
                        )))?;
                        current = parent;
                    }
                    _ => ()
                }
            }
            TokenKind::EndMessage => {
                while current.borrow().data.token != TokenKind::Message {
                    let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                        token.line,
                        String::from("Unmatched `]` or `end` symbol.")))?;
                    current = parent;
                }
                let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                    token.line,
                    String::from("Unmatched `]` symbol.")))?;
                current = parent;
            }
            TokenKind::EndList => {
                while current.borrow().data.token != TokenKind::List {
                    let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                        token.line,
                        String::from("Unmatched `)` symbol.")))?;
                    current = parent;
                }
                let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                    token.line,
                    String::from("Unmatched `)` symbol.")))?;
                current = parent;
            }
            TokenKind::EndOperation => {
                while current.borrow().data.token != TokenKind::Operation {
                    let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                        token.line,
                        String::from("Unmatched `>` symbol.")))?;
                    current = parent;
                }
                let parent = current.borrow().parent.clone().ok_or(Error::ParseError(
                    token.line,
                    String::from("Unmatched `>` symbol.")))?;
                current = parent;
            }
        }
    }
    Ok(current.borrow().flatten())
}
#[derive(Clone, Debug, PartialEq)]
pub struct AbstractToken {
    pub line: usize,
    pub token: AbstractKind,
}
pub type AToken = AbstractToken;
impl AToken {
    fn new(line: usize, token: AbstractKind) -> AToken {
        AToken {
            line,
            token
        }
    }
}
impl Display for AToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} : {:?}", self.line, self.token)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum AbstractKind {
    Root,
    Set(String),
    Get(String),
    Let(String),
    Number(f32),
    Integer(i32),
    Identifier(String),
    String(String),
    Character(char),
    Type(String, Vec<String>),
    Implement(String, Vec<String>, String),
    Block,
    Message,
    List,
    Expansion,
    Yield,
    Define(String, String),
    Lambda(Vec<String>),
    New,
    Self_,
    Unit,
}
pub type AKind = AbstractKind;

fn parse_abstract(cst: &FlatTree<Token>) -> Result<FlatTree<AbstractToken>> {
    match cst.data.token.clone() {
        TokenKind::Number(number) => {
            if number.contains(".") {
                Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Number(number.parse().map_err(
                    |_| Error::ParseError(cst.data.line, String::from("Failed to parse float.")))?
                )), Vec::new()))
            } else {
                Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Integer(number.parse().map_err(
                    |_| Error::ParseError(cst.data.line, String::from("Failed to parse integer.")))?
                )), Vec::new()))
            }
        }
        TokenKind::String(string) => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::String(string)), Vec::new())),
        TokenKind::Identifier(ident) => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Identifier(ident)), Vec::new())),
        TokenKind::Character(char) => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Character(char)), Vec::new())),
        TokenKind::New => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::New), Vec::new())),
        TokenKind::Unit => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Unit), Vec::new())),
        TokenKind::Self_ => Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Self_), Vec::new())),
        TokenKind::Message => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::Message),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::Block => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::Block),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::List => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::List),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::Operation => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::Expansion),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::Semicolon | TokenKind::EndBlock | TokenKind::EndMessage | TokenKind::EndList | TokenKind::EndOperation => Err(
            Error::ParseError(cst.data.line, String::from(
                "Unexpected token. This is likely an internal error with the compiler.\
                Tokens internally reffered to as `Semicolon`, `EndList`, `EndBlock`, `EndOperation`, or `EndMessage`\
                have snuck through the cracks and made it to the wrong stage of parsing."))),
        TokenKind::For => Err(Error::ParseError(cst.data.line, String::from("Unexpected keyword `for`."))),
        TokenKind::Implement => {
            let (TokenKind::Identifier(ref name) | TokenKind::String(ref name)) = cst.children.get(0).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected identifier or string after keyword `impl`."))
            )?.data.token else {
                return Err(Error::ParseError(cst.data.line, String::from("Expected identifier or string after keyword `impl`.")))
            };
            let mut args: Vec<String> = Vec::new();
            let mut taking_args: bool = true;
            let mut impl_type: String = String::new();
            let mut pivot: usize = 0;
            for (i, child) in cst.children[1..].iter().enumerate() {
                if child.data.token == TokenKind::For {
                    taking_args = false;
                    continue;
                }
                let (TokenKind::Identifier(ref arg) | TokenKind::String(ref arg)) = child.data.token else {
                    return Err(Error::ParseError(cst.data.line, String::from(
                        "Expected identifier, string, or `for` keyword in impl-definition in the form of a parameter or type name."
                    )));
                };
                if taking_args {
                    args.push(arg.clone());
                } else {
                    impl_type = arg.clone();
                    pivot = i + 2;
                    break;
                }
            }
            let mut children: Vec<FlatTree<AbstractToken>> = Vec::new();
            for child in cst.children[pivot..].iter() {
                children.push(parse_abstract(child)?);
            }
            Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Implement(name.clone(), args, impl_type)), children))
        }
        TokenKind::Lambda => {
            let mut args: Vec<String> = Vec::new();
            for child in cst.children[..cst.children.len() - 1].iter() {
                let (TokenKind::Identifier(ref arg) | TokenKind::String(ref arg)) = child.data.token else {
                    return Err(Error::ParseError(cst.data.line, String::from(
                        "Expected identifier or string after lambda definition (`λ`, `lambda`, `fn`, `func`, `function`, or `\\` keywords/operators) in the form of a parameter, the same as an `impl` parameter definition."
                    )));
                };
                args.push(arg.clone());
            }
            let body = parse_abstract(cst.children.last().ok_or(Error::ParseError(cst.data.line, String::from(
                "Unexpected lambda with no implementation or arguments"
            )))?)?;
            Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Lambda(args)), vec![body]))
        }
        TokenKind::Root => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::Root),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::Yield => Ok(FlatTree::new(
            AToken::new(cst.data.line, AKind::Yield),
            cst.children.iter().map(
                |child| parse_abstract(child)
            ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?)),
        TokenKind::Get => {
            let TokenKind::Identifier(ref name) = cst.children.get(0).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `get`."))
            )?.data.token else {
                return Err(Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `get`.")))
            };
            Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Get(name.clone())), Vec::new()))
        }
        TokenKind::Set => {
            let TokenKind::Identifier(ref name) = cst.children.get(0).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `set`."))
            )?.data.token else {
                return Err(Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `set`.")))
            };
            Ok(FlatTree::new(
                AToken::new(cst.data.line, AKind::Set(name.clone())),
                cst.children[1..].iter().map(
                    |child| parse_abstract(child)
                ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?))
        }
        TokenKind::Let => {
            let TokenKind::Identifier(ref name) = cst.children.get(0).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `let`."))
            )?.data.token else {
                return Err(Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `let`.")))
            };
            Ok(FlatTree::new(
                AToken::new(cst.data.line, AKind::Let(name.clone())),
                cst.children[1..].iter().map(
                    |child| parse_abstract(child)
                ).collect::<Result<Vec<FlatTree<AbstractToken>>>>()?))
        }
        TokenKind::Type => {
            let TokenKind::Identifier(ref name) = cst.children.get(0).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `type`."))
            )?.data.token else {
                return Err(Error::ParseError(cst.data.line, String::from("Expected identifier after keyword `type`.")))
            };
            let block = cst.children.get(1).ok_or(
                Error::ParseError(cst.data.line, String::from("Expected block after keyword `type`"))
            )?;
            let mut fields: Vec<String> = Vec::new();
            for child in block.children.iter() {
                let TokenKind::Identifier(ref name) = child.data.token else {
                    return Err(Error::ParseError(cst.data.line, String::from("Invalid field name.")))
                };
                fields.push(name.clone());
            }
            Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Type(name.clone(), fields)), Vec::new()))
        }
        TokenKind::Define => {
            let (TokenKind::Identifier(ref name) | TokenKind::String(ref name)) = cst.children.get(0).ok_or(Error::ParseError(
                cst.data.line,
                String::from("`def` block missing identifier or string as expansion name")
            ))?.data.token else {
                return Err(Error::ParseError(
                    cst.data.line,
                    String::from("`def` block missing identifier or string as expansion name")
                ));
            };
            let (TokenKind::Identifier(ref exp) | TokenKind::String(ref exp)) = cst.children.get(1).ok_or(Error::ParseError(
                cst.data.line,
                String::from("`def` block missing identifier or string as expansion value after expansion name")
            ))?.data.token else {
                return Err(Error::ParseError(
                    cst.data.line,
                    String::from("`def` block missing identifier or string as expansion value after expansion name")
                ));
            };
            let exp: String = exp.clone().replace("\\n", "\n").replace("\\t", "\t");
            return Ok(FlatTree::new(AToken::new(cst.data.line, AKind::Define(name.clone(), exp)), Vec::new()));
        }
    }
}

pub fn parse(program: String) -> Result<FlatTree<AToken>> {
    let tokens = tokenize(program);
    let cst = parse_concrete(tokens)?;
    let ast = parse_abstract(&cst)?;
    Ok(ast)
}
