use std::collections::{HashMap, HashSet};
use super::*;
use crate::parse::{AKind, FlatTree, AToken};

struct Info {
    scope: HashMap<String, u32>,
    file: String,
    class: String,
    message: String,
    lambdas: usize,
    types: HashMap<String, HashMap<String, u32>>,
    aliases: HashMap<String, String>,
    implementations: HashMap<String, Vec<String>>,
    expansions: HashMap<String, String>,
    labels: usize,
    namespace: String,
    imports: Vec<String>,
    locals: u32,
}

pub fn compile_program(ast: FlatTree<AToken>, file: String) -> Result<Vec<(String, String)>> {
    let mut info: Info = Info {
        scope: HashMap::new(),
        file: String::from("prelude.rad"),
        class: String::new(),
        message: String::new(),
        lambdas: 0,
        types: HashMap::new(),
        aliases: HashMap::new(),
        implementations: HashMap::new(),
        expansions: HashMap::new(),
        labels: 0,
        namespace: String::new(),
        imports: Vec::new(),
        locals: 0,
    };
    let FlatTree { children, data: AToken { token: AKind::Root, .. } } = ast else {
        return Err(Error::CompileError(0, file, String::from(
            "Root missing for abstract syntax tree (Internal Error). \
            This may be caused by improperly placed keywords as well as an internal bug. \
            Check there aren't any keywords that aren't properly closed \
            (like an `impl` block without braces after it)"
        )));
    };
    {
        let program = crate::PRELUDE.to_string();
        let ast: FlatTree<parse::AToken> = parse::parse(program, String::from("prelude.rad"))?;
        compile_special(ast, String::from("prelude.rad"), &mut info)?;
    }
    info.file = file.clone();
    for child in children {
        compile_top(child, &mut info)?;
    }
    let mut files: Vec<(String, String)> = Vec::new();
    for (type_name, implementations) in info.implementations {
        let native: bool = info.types.contains_key(&type_name);
        let mut contents: String = String::new();
        let path: String;
        if native {
            contents.push_str(
                format!(
                    "type object \"{type_name}\" {} #\n\n",
                    info.types[&type_name].len()
                ).as_str()
            );
            path = format!("Radia.{file}.{type_name}.oovmt");
        } else {
            contents.push_str(format!("mod object \"{type_name}\" 0 #\n\n").as_str());
            path = format!("Radia.{file}.{type_name}.oovmm");
        }
        for implementation in implementations {
            contents.push_str(implementation.as_str());
            contents.push('\n');
        }
        files.push((path
                        .replace("<", ".oovm.")
                        .replace(">", ".magic.")
                        .replace("::", ".ns.")
                        .replace("..",".")
                    , contents));
    }
    Ok(files)
}
fn compile_special(ast: FlatTree<AToken>, file: String, info: &mut Info) -> Result<()> {
    let FlatTree { children, data: AToken { token: AKind::Root, .. } } = ast else {
        return Err(Error::CompileError(0, file, String::from(
            "Root missing for abstract syntax tree (Internal Error). \
            This may be caused by improperly placed keywords as well as an internal bug. \
            Check there aren't any keywords that aren't properly closed \
            (like an `impl` block without braces after it)"
        )));
    };
    for child in children {
        compile_top(child, info)?;
    }
    Ok(())
}

fn compile_top(ast: FlatTree<AToken>, info: &mut Info) -> Result<()> {
    match ast.data.token {
        AKind::Type(name, fields) => {
            let mut fields_map = HashMap::new();
            for (i, field) in fields.into_iter().enumerate() {
                fields_map.insert(field, i as u32);
            }
            info.aliases.insert(name.clone(), format!("{}::{name}", info.namespace));
            info.types.insert(format!("{}::{name}", info.namespace), fields_map);
        }
        AKind::Implement(msg, args, mut type_name) => {
            if info.aliases.contains_key(&type_name) {
                type_name = info.aliases[&type_name].clone();
            }
            let mut output: String = format!("method \"{msg}\" {} {{\n", args.len());
            info.lambdas = 0;
            info.message = msg.clone();
            info.scope = HashMap::new();
            info.class = type_name.clone();
            info.labels = 0;
            info.locals = 0;
            for (i, arg) in args.into_iter().enumerate() {
                info.scope.insert(arg, i as u32);
                info.locals += 1;
            }
            let children: Vec<FlatTree<AToken>>;
            let first = ast.children.get(0).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("`impl` block missing implementation")
            ))?;
            if let AKind::Block = first.data.token {
                children = first.children.clone();
            } else {
                children = ast.children;
            }
            for child in children {
                output.push_str(compile(child, info)?.as_str());
            }
            output.push_str("\tret\n}\n");
            let mut implementations = info.implementations.get(&type_name).unwrap_or(&Vec::new()).clone();
            implementations.push(output);
            info.implementations.insert(type_name, implementations);
        }
        AKind::Define(name, expansion) => {
            info.expansions.insert(name, expansion);
        }
        AKind::Namespace(name) => {
            info.namespace = name;
        }
        AKind::Include(path) => {
            let namespace = info.namespace.clone();
            let program = std::fs::read_to_string(path).map_err(Error::IOError)?;
            let ast: FlatTree<parse::AToken> = parse::parse(program, info.file.clone())?;
            compile_special(ast, String::from("prelude.rad"), info)?;
            info.namespace = namespace;
        }
        token_kind => return Err(Error::CompileError(
            ast.data.line,
            info.file.clone(),
            format!(
                "Only `impl` blocks, expansion definitions (`def` statements), \
                `namespace` declarations, and `type` definitions may be top-level: {:?}",
                token_kind
            )
        )),
    }
    Ok(())
}

fn compile(ast: FlatTree<AToken>, info: &mut Info) -> Result<String> {
    match ast.data.token {
        AKind::Get(field_name) => {
            let offset = info.types.get(&info.class).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Cannot `get` field for external or unknown types")
            ))?.get(&field_name).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Unknown field `{field_name}` for type `{}`", info.class)
            ))?.clone();
            Ok(format!("\tget {offset}\n"))
        }
        AKind::Set(field_name) => {
            let offset = info.types.get(&info.class).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Cannot `get` field for external or unknown types")
            ))?.get(&field_name).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Unknown field `{field_name}` for type `{}`", info.class)
            ))?.clone();
            let val = compile(ast.children.get(0).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Keyword `set` missing value.")
            ))?.clone(), info)?;
            Ok(format!("{val}\tset {offset}\n"))
        }
        AKind::Yield => {
            let val = compile(ast.children.get(0).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Keyword `yield` missing value")
            ))?.clone(), info)?;
            Ok(format!("{val}\tret\n"))
        }
        AKind::Message => {
            let mut comps: Vec<String> = Vec::new();
            for child in ast.children[2..].iter() {
                comps.push(compile(child.clone(), info)?);
            }
            let mut output: String = String::new();
            for comp in comps {
                output.push_str(comp.as_str());
            }
            let obj = ast.children.get(0).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!(
                    "Empty message send not allowed \
                    (lists are defined with `(a b c)`, not `[a b c]`)"
                )
            ))?.clone();
            let msg: String;
            match ast.children.get(1).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Keyword `set` missing value")
            ))?.data.token {
                AKind::String(ref message)
                | AKind::Identifier(ref message) => {
                    msg = message.clone();
                }
                _ => return Err(Error::CompileError(
                    ast.data.line,
                    info.file.clone(),
                    format!(
                        "Invalid message (must be in form of string literal or identifier literal)."
                    )
                ))
            }
            if let AKind::New = obj.data.token {
                let obj_type = info.aliases.get(&msg).unwrap_or(&msg);
                Ok(format!("{output}\tnew \"{obj_type}\"\n\tsend \"{msg}\"\n"))
            }
            else {
                let obj = compile(obj, info)?;
                Ok(format!("{output}{obj}\tsend \"{msg}\"\n"))
            }
        }
        AKind::Unit => Ok(String::from("\tmain\n")),
        AKind::Self_ => Ok(String::from("\tthis\n")),
        AKind::Identifier(ident) => {
            let local = info.scope.get(&ident).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Unknown local variable `{ident}`")
            ))?.clone();
            Ok(format!("\tload {local} ref\n"))
        }
        AKind::Let(ident) => {
            let val = compile(ast.children.get(0).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!("Keyword `let` missing value")
            ))?.clone(), info)?;
            let local = info.locals;
            info.locals += 1;
            info.scope.insert(ident.clone(), local);
            Ok(format!("{val}\tlocal {local}\n"))
        }
        AKind::Number(num) => {
            Ok(format!("\tmint {num:?}\n"))
        }
        AKind::Integer(num) => {
            Ok(format!("\tmint {num:?}\n"))
        }
        AKind::Character(char) => {
            Ok(format!("\tmint {}\n", char as u32))
        }
        AKind::List => {
            let mut output: String = String::new();
            let len = ast.children.len();
            for child in ast.children {
                output.push_str(compile(child.clone(), info)?.as_str());
            }
            Ok(format!("{output}\tmarr {}\n", len))
        }
        AKind::String(str) => {
            Ok(format!("\tlstr \"{}\"\n", str
                .replace("\\n", "\\0a")
                .replace("\\t", "\\09")
                .replace("\\\\", "\\5c")
            ))
        }
        AKind::Block => {
            let mut output: String = String::new();
            let old_scope = info.scope.clone();
            for child in ast.children {
                output.push_str(compile(child.clone(), info)?.as_str());
            }
            info.scope = old_scope;
            Ok(output)
        }
        AKind::Lambda(ref args) => {
            let mut reservations: Vec<String> = Vec::new();
            reserve(&ast, info, &mut reservations);
            let old_scope = info.scope.clone();
            info.scope = HashMap::new();
            info.scope.insert(String::from("f"), 0);
            for (i, reservation) in reservations.iter().enumerate() {
                info.scope.insert(reservation.clone(), i as u32 + 1);
            }
            for arg in args {
                info.scope.insert(arg.clone(), info.scope.len() as u32);
            }
            info.locals = info.scope.len() as u32;
            let lambda = info.lambdas;
            info.lambdas += 1;
            let lambda_name = format!("{} @ {} $ {lambda}", info.file, info.message);
            let mut output = format!("\tthis ref\n\tlstr \"{lambda_name}\"\n");
            for reservation in reservations.iter() {
                output.push_str(format!("\tload {} ref\n", old_scope[reservation]).as_str());
            }
            output.push_str(format!("\tmarr {}\n", reservations.len()).as_str());
            output.push_str("\tnew \"Function\"\n\tsend \"Function\"\n");
            let mut lambda: String = format!(
                "method \"{lambda_name}\" {} {{\n",
                reservations.len() + args.len() + 1
            );
            for child in ast.children {
                lambda.push_str(compile(child, info)?.as_str());
            }
            lambda.push_str("\tret\n}\n");
            let mut implementations = info
                .implementations
                .get(&info.class)
                .unwrap_or(&Vec::new())
                .clone();
            implementations.push(lambda);
            info.implementations.insert(info.class.clone(), implementations);
            info.scope = old_scope;
            Ok(output)
        }
        AKind::Expansion => {
            let (AKind::Identifier(ref name) | AKind::String(ref name)) = ast
                .children
                .get(0)
                .ok_or(Error::CompileError(
                    ast.data.line,
                    info.file.clone(),
                    format!(
                        "Empty angled brackets are disallowed. \
                        For the type `<>` of `unit`, use \"<>\" with the double quotes included. \
                        A correct example of angled brackets would be: `<Add.Int 3 5>`, \
                        which would add the integers three and five."
                    )
                ))?
                .data
                .token
            else {
                return Err(Error::CompileError(
                    ast.data.line,
                    info.file.clone(),
                    format!(
                        "Invalid expansion name. \
                        Correct expansion syntax is of the form: `<expansion-name arg1 arg2 argN>` \
                        or `<\"quoted expansion name\" arg1 arg2 argN>`"
                    )
                ));
            };
            let mut expansion = info.expansions.get(name).ok_or(Error::CompileError(
                ast.data.line,
                info.file.clone(),
                format!(
                    "Unknown expansion `{name}`. \
                    If this is unexpected, make sure the \
                    intended expansion is defined *before* being used."
                )
            ))?.clone();
            if expansion.contains("$label") {
                let label_id = info.labels;
                info.labels += 1;
                expansion = expansion.replace("$label", label_id.to_string().as_str());
            }
            let mut comps: Vec<String> = Vec::new();
            for child in ast.children[1..].iter() {
                comps.push(compile(child.clone(), info)?);
            }
            for (i, comp) in comps.iter().enumerate() {
                expansion = expansion.replace(format!("${i}").as_str(), comp);
            }
            Ok(expansion)
        }
        token_kind => Err(Error::NotImplemented(format!("{:?}", token_kind))),
    }
}

fn reserve(ast: &FlatTree<AToken>, info: &mut Info, reservations: &mut Vec<String>) -> () {
    match ast.data.token {
        AKind::Identifier(ref ident) => {
            if info.scope.contains_key(ident) {
                if !reservations.contains(&ident) {
                    reservations.push(ident.clone());
                }
            }
        }
        _ => {
            for child in ast.children.iter() {
                reserve(&child, info, reservations);
            }
        }
    }
}