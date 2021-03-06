//     Copyright 2019 Haoran Wang
//
//     Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
//     You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
//     distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//     See the License for the specific language governing permissions and
//     limitations under the License.
// ------------------------------------------------------------------------
// parser.rs:
//           try to support all c11 features, version 0.1
// ------------------------------------------------------------------------

use crate::ast::{ConstantType, NodeType, ParseNode};
use crate::lexer;
use crate::sema;
use crate::symtable::{BaseType, TypeExpression};

// XXX: How to handle error message properly should be improved later
//      and some uncommon situations support should be added.

// ------------------------------------------------------------------------
// helper function
// ------------------------------------------------------------------------
fn error_handler(expect: &str, toks: &lexer::TokType, pos: usize) -> String {
    // return a detailed error message.
    // now it could be simple, just print the token information
    return format!("Expected `{}`, found {:?} at {}", expect, toks, pos);
}

fn check_tok(pos: usize, toks: &[lexer::TokType], expect: &lexer::TokType) -> Result<(), String> {
    check_pos(pos, toks.len())?;

    if &toks[pos] != expect {
        return Err(format!("Expected: {:?}, found {:?}", expect, toks[pos]));
    }

    return Ok(());
}

fn check_pos(pos: usize, toks_len: usize) -> Result<(), String> {
    if pos >= toks_len {
        return Err(format!("out of token index"));
    }
    return Ok(());
}

fn p_identifier(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::IDENTIFIER(val) => {
            let mut cur_node = ParseNode::new(NodeType::Identifier(val.to_string()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Identifier(val.to_string()));
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("identifier", &toks[pos], pos));
        }
    }
}

// primary_expression
// 	: IDENTIFIER
// 	| constant
// 	| string
// 	| '(' expression ')'
// 	| generic_selection
// 	;

fn p_primary_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::PrimaryExpression);
    if let Ok((child_node, new_pos)) = p_identifier(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, new_pos));
    } else if let Ok((child_node, new_pos)) = p_constant(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, new_pos));
    } else if let Ok((child_node, new_pos)) = p_string(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, new_pos));
    } else if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
        let pos = pos + 1;
        let (child_node, pos) = p_expression(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        check_tok(pos, &toks, &lexer::TokType::RParen)?;
        let pos = pos + 1;
        return Ok((cur_node, pos));
    } else if let Ok((child_node, new_pos)) = p_generic_selection(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, new_pos));
    } else {
        return Err(format!("Can not parse primary expression"));
    }
}

// constant
// 	: IConstant		/* includes character_constant */
// 	| FConstant
// 	| EnumerationConstant	/* after it has been defined as such */
// 	;
fn p_constant(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::IConstant(i_val) => {
            let mut cur_node = ParseNode::new(NodeType::Constant(ConstantType::I64(*i_val)));
            // cause if the value was assigned to int, we can easily cast long to int.
            cur_node.type_exp = TypeExpression::new_val(BaseType::Long);
            Ok((cur_node, pos + 1))
        }
        lexer::TokType::FConstant(f_val) => {
            let mut cur_node = ParseNode::new(NodeType::Constant(ConstantType::F64(*f_val)));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Double);
            Ok((cur_node, pos + 1))
        }
        lexer::TokType::EnumerationConstant(e_val) => {
            // XXX: this need to be processed by the lexer maybe
            let mut cur_node =
                ParseNode::new(NodeType::Constant(ConstantType::String(e_val.to_string())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Long);
            Ok((cur_node, pos + 1))
        }
        _ => Err(error_handler("constant", &toks[pos], pos)),
    }
}

// enumeration_constant		/* before it has been defined as such */
// 	: IDENTIFIER
// 	;
// TODO: should judge whether a identifier is a enumeration_constant in semantics_analyzer
fn p_enumeration_constant(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::IDENTIFIER(name) => {
            let mut cur_node = ParseNode::new(NodeType::EnumerationConstant(name.to_string()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Identifier(name.to_string()));
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("identifier", &toks[pos], pos));
        }
    }
}

// string
// 	: StringLiteral
// 	| FuncName
// 	;
fn p_string(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::StringLiteral(v, _tag) => {
            let mut cur_node = ParseNode::new(NodeType::STRING(v.to_string()));
            let len = v.len();
            let mut t_exp = TypeExpression::new_val(BaseType::Array(len));
            t_exp.val.push(BaseType::Char);
            cur_node.type_exp = t_exp;
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::FuncName => {
            // FIXME: cause now there's no semantic analyzer, so just pass the literal
            let mut cur_node = ParseNode::new(NodeType::STRING("__func_name__".to_string()));
            let len = "__func_name__".len();
            let mut t_exp = TypeExpression::new_val(BaseType::Array(len));
            t_exp.val.push(BaseType::Char);
            cur_node.type_exp = t_exp;
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("String literal", &toks[pos], pos));
        }
    }
}

// generic_selection
// 	: GENERIC '(' assignment_expression ',' generic_assoc_list ')'
// 	;
// TODO: Add type system for this kind of node
fn p_generic_selection(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::GenericSelection);

    if toks[pos] != lexer::TokType::GENERIC {
        return Err(error_handler("__Generic", &toks[pos], pos));
    }
    let pos = pos + 1;

    check_tok(pos, &toks, &lexer::TokType::LParen)?;

    let pos = pos + 1;
    check_pos(pos, toks.len())?;
    let (child_node, pos) = p_assignment_expression(toks, pos)?;
    cur_node.child.push(child_node);

    check_tok(pos, &toks, &lexer::TokType::Comma)?;
    let pos = pos + 1;
    let (child_node, pos) = p_generic_assoc_list(toks, pos)?;
    cur_node.child.push(child_node);

    check_tok(pos, &toks, &lexer::TokType::RParen)?;
    let pos = pos + 1;

    return Ok((cur_node, pos));
}

// generic_assoc_list
// 	: generic_association
// 	| generic_assoc_list ',' generic_association
// 	;
// EBNF:
// -> generic_association { ',' generic_association }
// TODO: Add type system for this kind of node
fn p_generic_assoc_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::GenericAssocList);

    let (child_node, pos) = p_generic_association(toks, pos)?; // if error, then out

    cur_node.child.push(child_node);

    // let mut back_pos = pos;
    let mut pos = pos;
    while let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
        let back_pos = pos;
        pos = pos + 1;
        match p_generic_association(toks, pos) {
            Ok((child_node, tmp)) => {
                cur_node.child.push(child_node);
                pos = tmp;
            }
            Err(_) => {
                pos = back_pos;
                break;
            }
        }
    }

    return Ok((cur_node, pos));
}

// generic_association
// 	: type_name ':' assignment_expression
// 	| DEFAULT ':' assignment_expression
// TODO: Add type system for this kind of node
fn p_generic_association(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    if pos >= toks.len() {
        return Err(format!("out of token index"));
    }

    let mut cur_node = ParseNode::new(NodeType::GenericAssociation);
    let mut pos = pos;
    if let Ok((child_node, tmp_pos)) = p_type_name(toks, pos) {
        pos = tmp_pos;
        cur_node.child.push(child_node);
    } else if toks[pos] == lexer::TokType::DEFAULT {
        pos = pos + 1;
    } else {
        return Err(format!(
            "Can't find proper type name or default, found {:?} at {}",
            toks[pos], pos
        ));
    }

    check_tok(pos, &toks, &lexer::TokType::Colon)?;
    let pos = pos + 1;
    let (child_node, pos) = p_assignment_expression(toks, pos)?;
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// postfix_expression
// 	: primary_expression
// 	| postfix_expression '[' expression ']'
// 	| postfix_expression '(' ')'
// 	| postfix_expression '(' argument_expression_list ')'
// 	| postfix_expression '.' IDENTIFIER
// 	| postfix_expression PtrOp IDENTIFIER
// 	| postfix_expression IncOp
// 	| postfix_expression DecOp
// 	| '(' type_name ')' '{' initializer_list '}'
// 	| '(' type_name ')' '{' initializer_list ',' '}'
// 	;
// let's define:
// pre
//  : primary_expression
// 	| '(' type_name ')' '{' initializer_list '}'
// 	| '(' type_name ')' '{' initializer_list ',' '}'
//
// Then, transfer to:
//
// postfix_expression:
//  pre { postfix_expression_post }
// TODO: Need carefully review
fn p_postfix_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::PostfixExpression);

    if let Ok((child_node, pos)) = p_primary_expression(toks, pos) {
        let pre_type = child_node.type_exp.clone();

        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        let mut pos = pos;
        let mut inc = 0;
        loop {
            if let Ok((child_node, tmp_pos)) = p_postfix_expression_post(toks, pos) {
                inc = inc + 1;
                cur_node.child.push(child_node);
                pos = tmp_pos;
            } else {
                break;
            }
        }

        if inc == 0 {
            cur_node.type_exp = pre_type;
        }
        return Ok((cur_node, pos));
    } else if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
        let pos = pos + 1;
        let (child_node, pos) = p_type_name(toks, pos)?;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);

        check_tok(pos, &toks, &lexer::TokType::RParen)?;
        let pos = pos + 1;

        check_tok(pos, &toks, &lexer::TokType::LBrace)?;
        let pos = pos + 1;

        let (child_node, pos) = p_initializer_list(toks, pos)?;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBrace) {
            let pos = pos + 1;

            let mut pos = pos;
            loop {
                if let Ok((child_node, tmp_pos)) = p_postfix_expression_post(toks, pos) {
                    cur_node.type_exp.child.push(child_node.type_exp.clone());
                    cur_node.child.push(child_node);
                    pos = tmp_pos;
                } else {
                    break;
                }
            }
            return Ok((cur_node, pos));
        } else {
            check_tok(pos, &toks, &lexer::TokType::Comma)?;
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::RBrace)?;
            let pos = pos + 1;

            let mut pos = pos;
            loop {
                if let Ok((child_node, tmp_pos)) = p_postfix_expression_post(toks, pos) {
                    cur_node.type_exp.child.push(child_node.type_exp.clone());
                    cur_node.child.push(child_node);
                    pos = tmp_pos;
                } else {
                    break;
                }
            }
            return Ok((cur_node, pos));
        }
    } else {
        return Err(format!("Error parse postfix_expression"));
    }
}

// postfix_expression_post
//  : '[' expression ']'
// 	| '(' ')'
// 	| '(' argument_expression_list ')'
// 	| '.' IDENTIFIER
// 	| PtrOp IDENTIFIER
// 	| IncOp
// 	| DecOp
// TODO: Need carefully review
// TODO: Add type system for this kind of node
fn p_postfix_expression_post(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::LBracket => {
            let mut cur_node = ParseNode::new(NodeType::PostfixExpressionPost(toks[pos].clone()));
            let pos = pos + 1;
            let (child_node, pos) = p_expression(toks, pos)?;
            cur_node.type_exp = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            check_tok(pos, &toks, &lexer::TokType::RBracket)?;
            let pos = pos + 1;
            return Ok((cur_node, pos));
        }
        lexer::TokType::LParen => {
            let mut cur_node = ParseNode::new(NodeType::PostfixExpressionPost(toks[pos].clone()));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RParen) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                let (child_node, pos) = p_argument_expression_list(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                check_tok(pos, &toks, &lexer::TokType::RParen)?;
                let pos = pos + 1;
                return Ok((cur_node, pos));
            }
        }
        lexer::TokType::Dot | lexer::TokType::PtrOp => {
            let mut cur_node = ParseNode::new(NodeType::PostfixExpressionPost(toks[pos].clone()));
            let pos = pos + 1;
            let (child_node, pos) = p_identifier(toks, pos)?;
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        }
        lexer::TokType::IncOp | lexer::TokType::DecOp => {
            let cur_node = ParseNode::new(NodeType::PostfixExpressionPost(toks[pos].clone()));
            let pos = pos + 1;
            return Ok((cur_node, pos));
        }
        _ => {
            return Err(format!("{:?} at {} is a postfix operator", toks[pos], pos));
        }
    }
}

// argument_expression_list
// 	: assignment_expression
// 	| argument_expression_list ',' assignment_expression
// 	;
// -> assignment_expression { ',' assignment_expression }
fn p_argument_expression_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::ArgumentExpressionList);

    let (child_node, pos) = p_assignment_expression(toks, pos)?; // if error, then out
    let pre_type = child_node.type_exp.clone();

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    let mut inc = 0;
    let mut pos = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        }
        match p_assignment_expression(toks, pos + 1) {
            Ok((child_node, tmp)) => {
                inc = inc + 1;
                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp;
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }
    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}

// unary_expression
// 	: postfix_expression
// 	| IncOp unary_expression
// 	| DecOp unary_expression
// 	| unary_operator cast_expression
// 	| SIZEOF unary_expression
// 	| SIZEOF '(' type_name ')'
// 	| ALIGNOF '(' type_name ')'
// 	;
fn p_unary_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match toks[pos] {
        lexer::TokType::IncOp | lexer::TokType::DecOp => {
            let mut cur_node = ParseNode::new(NodeType::UnaryExpression(Some(toks[pos].clone())));
            let pos = pos + 1;
            let (child_node, pos) = p_unary_expression(toks, pos)?;
            cur_node.type_exp = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        }
        lexer::TokType::SIZEOF => {
            // assign the return type of sizeof() to size_t
            let pos = pos + 1;
            let mut cur_node = ParseNode::new(NodeType::UnaryExpression(Some(toks[pos].clone())));
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
                let (child_node, pos) = p_type_name(toks, pos)?;
                cur_node.type_exp = TypeExpression::new_val(BaseType::SizeT);
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else {
                let (child_node, pos) = p_unary_expression(toks, pos)?;
                cur_node.type_exp = TypeExpression::new_val(BaseType::SizeT);
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            }
        }
        lexer::TokType::ALIGNOF => {
            // should return type size_t
            let mut cur_node = ParseNode::new(NodeType::UnaryExpression(Some(toks[pos].clone())));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
                let pos = pos + 1;
                let (child_node, pos) = p_type_name(toks, pos)?;
                cur_node.type_exp = TypeExpression::new_val(BaseType::SizeT);
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else {
                return Err(error_handler("(", &toks[pos], pos));
            }
        }
        _ => {
            // postfix_expression
            // unary_operator cast_expression
            if let Ok((child_node, pos)) = p_unary_operator(toks, pos) {
                let mut cur_node = ParseNode::new(NodeType::UnaryExpression(None));
                let unary_op = if let NodeType::UnaryOperator(op) = child_node.entry.clone() {
                    op
                } else {
                    return Err(format!(
                        "Syntax: expected unary_operator, got {:?}",
                        child_node.entry
                    ));
                };

                cur_node.child.push(child_node);
                let (child_node, pos) = p_cast_expression(toks, pos)?;
                match unary_op {
                    lexer::TokType::SingleAnd => {
                        cur_node.type_exp = TypeExpression::new_val(BaseType::Pointer);
                    }
                    lexer::TokType::Multi => {
                        // *(void *) , need to be casted.
                        let mut t_exp = TypeExpression::new_val(BaseType::Pointer);
                        t_exp.val.push(BaseType::VoidPointer);
                        cur_node.type_exp = t_exp;
                    }
                    _ => {
                        cur_node.type_exp = child_node.type_exp.clone();
                    }
                }
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else if let Ok((child_node, pos)) = p_postfix_expression(toks, pos) {
                let mut cur_node = ParseNode::new(NodeType::UnaryExpression(None));
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else {
                return Err(format!("Can't parse unary_expression"));
            }
        }
    }
}

// unary_operator
// 	: '&'
// 	| '*'
// 	| '+'
// 	| '-'
// 	| '~'
// 	| '!'
// 	;
fn p_unary_operator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    // need to match
    match &toks[pos] {
        lexer::TokType::Minus|
        lexer::TokType::SingleAnd| // '&', different with '&&' as TokType::And
        lexer::TokType::Multi|
        lexer::TokType::Exclamation| // '!'
        lexer::TokType::Tilde |
        lexer::TokType::Plus => {
            // don't have type, just care about the operator type
            return Ok((ParseNode::new(NodeType::UnaryOperator(toks[pos].clone())), pos + 1));
        }
        _ => {
            return Err(error_handler("unary_operator", &toks[pos], pos));
        }
    }
}

// cast_expression
// 	: unary_expression
// 	| '(' type_name ')' cast_expression
// 	;
fn p_cast_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::CastExpression);
    if let Ok((child_node, pos)) = p_unary_expression(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
        let (child_node, pos) = p_type_name(toks, pos + 1)?;
        let to_type = child_node.type_exp.clone();

        cur_node.child.push(child_node);

        check_tok(pos, &toks, &lexer::TokType::RParen)?;

        let (child_node, pos) = p_cast_expression(toks, pos)?;
        let from_type = child_node.type_exp.clone();

        if sema::judge_cast(&to_type, &from_type) == false {
            return Err(format!(
                "Can not cast from {:?} to {:?}",
                from_type, to_type
            ));
        }

        cur_node.type_exp = to_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse cast_expression"));
    }
}

// multiplicative_expression
// 	: cast_expression
// 	| multiplicative_expression '*' cast_expression
// 	| multiplicative_expression '/' cast_expression
// 	| multiplicative_expression '%' cast_expression
// 	;
//   cast_expression { ('*' | '/' | '%') cast_expression }
fn p_multiplicative_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::MultiplicativeExpression);
    // exp -> multiplicative_expression
    let mut pos = pos;
    let (child_node, tmp_pos) = p_cast_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    pos = tmp_pos;
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::Mod
        && *tok != lexer::TokType::Multi
        && *tok != lexer::TokType::Splash
    {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::Mod
        || *tok == lexer::TokType::Multi
        || *tok == lexer::TokType::Splash
    {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_cast_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();

        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}
// additive_expression
// 	: multiplicative_expression { ("+" | "-") multiplicative_expression }
// 	;
fn p_additive_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::AdditiveExpression);
    // exp -> multiplicative_expression
    let mut pos = pos;
    let (child_node, tmp_pos) = p_multiplicative_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    pos = tmp_pos;
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::Plus && *tok != lexer::TokType::Minus {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    //peek next token, if it is lexer::TokType::Plus or lexer::TokType::Minus
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::Plus || *tok == lexer::TokType::Minus {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_multiplicative_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}
// shift_expression
// 	: additive_expression
// 	| shift_expression LeftOp additive_expression
// 	| shift_expression RightOp additive_expression
// 	;
// -> additive_expression { (LeftOp | RightOp) additive_expression }
fn p_shift_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::ShiftExpression);
    // exp -> additive_expression
    let (child_node, pos) = p_additive_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::LeftOp && *tok != lexer::TokType::RightOp {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    // peek next token, if it is lexer::TokType::LeftOp or lexer::TokType::RightOp
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::LeftOp || *tok == lexer::TokType::RightOp {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_additive_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}
// relational_expression
// 	: shift_expression
// 	| relational_expression '<' shift_expression
// 	| relational_expression '>' shift_expression
// 	| relational_expression LeOp shift_expression
// 	| relational_expression GeOp shift_expression
// 	;
// -> shift_expression { ('<' | '>' | LeOp | GeOp) shift_expression }
fn p_relational_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::RelationalExpression);
    // exp -> shift_expression
    let (child_node, pos) = p_shift_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::Lt
        && *tok != lexer::TokType::Gt
        && *tok != lexer::TokType::GeOp
        && *tok != lexer::TokType::LeOp
    {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::LeOp
        || *tok == lexer::TokType::GeOp
        || *tok == lexer::TokType::Lt
        || *tok == lexer::TokType::Gt
    {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_shift_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// equality_expression
// 	: relational_expression
// 	| equality_expression EqOp relational_expression
// 	| equality_expression NeOp relational_expression
// 	;
// -> relational_expression { (EqOp | NeOp) relational_expression }
fn p_equality_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::EqualityExpression);
    // exp -> relational_expression
    let (child_node, pos) = p_relational_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::EqOp && *tok != lexer::TokType::NeOp {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::EqOp || *tok == lexer::TokType::NeOp {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_relational_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// and_expression
// 	: equality_expression
// 	| and_expression '&' equality_expression
// 	;
//  -> equality_expression { '&' equality_expression }
// XXX:
fn p_and_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::AndExpression);
    // exp -> equality_expression
    let (child_node, pos) = p_equality_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::SingleAnd {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::SingleAnd {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_equality_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// exclusive_or_expression
// 	: and_expression
// 	| exclusive_or_expression '^' and_expression
// 	;
//  -> and_expression { '^' and_expression }
fn p_exclusive_or_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::ExclusiveOrExpression);
    // exp -> and_expression
    let (child_node, pos) = p_and_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::ExclusiveOr {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::ExclusiveOr {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_and_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// inclusive_or_expression
// 	: exclusive_or_expression
// 	| inclusive_or_expression '|' exclusive_or_expression
// 	;
//  -> exclusive_or_expression { '|' exclusive_or_expression }
fn p_inclusive_or_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::InclusiveOrExpression);
    // exp -> exclusive_or_expression
    let (child_node, pos) = p_exclusive_or_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::InclusiveOr {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::InclusiveOr {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_exclusive_or_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// logical_and_expression
// 	: inclusive_or_expression
// 	| logical_and_expression AndOp inclusive_or_expression
// 	;
//  -> inclusive_or_expression { AndOp inclusive_or_expression }
fn p_logical_and_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::LogicalAndExpression);
    // exp -> inclusive_or_expression
    let (child_node, pos) = p_inclusive_or_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::AndOp {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::AndOp {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_inclusive_or_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// logical_or_expression
// 	: logical_and_expression
// 	| logical_or_expression OrOp logical_and_expression
// 	;
//  -> logical_and_expression { OrOp logical_and_expression }
fn p_logical_or_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::LogicalOrExpression);
    // exp -> logical_and_expression
    let (child_node, pos) = p_logical_and_expression(toks, pos)?;
    let mut l_type = child_node.type_exp.clone();
    let mut tok = &toks[pos];
    if *tok != lexer::TokType::OrOp {
        cur_node.type_exp = l_type;
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
    // exp -> BinaryExpression()
    let mut child_node = child_node;
    let mut pos = pos;
    while *tok == lexer::TokType::OrOp {
        let mut bincur_node = ParseNode::new(NodeType::BinaryExpression(tok.clone()));
        pos = pos + 1;
        let op = tok.clone();
        let (next_child_node, tmp_pos) = p_logical_and_expression(toks, pos)?;
        let r_type = next_child_node.type_exp.clone();
        pos = tmp_pos;
        bincur_node.child.push(child_node);
        bincur_node.child.push(next_child_node);
        if let (true, combine_type) = sema::judge_combine_type(&l_type, &r_type, &op) {
            bincur_node.type_exp = combine_type;
        } else {
            return Err(format!(
                "can not use type: {:?} to {:?} type {:?}, ",
                l_type, op, r_type
            ));
        }
        child_node = bincur_node;
        l_type = child_node.type_exp.clone();
        tok = &toks[pos];
    }
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    return Ok((cur_node, pos));
}

// conditional_expression
// 	: logical_or_expression
// 	| logical_or_expression '?' expression ':' conditional_expression
// 	;
fn p_conditional_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    // XXX: should make sure expression and conditional_expression are the same type.
    //      the final conditional expression type would be expression type,
    //      and also have to make sure logical_or_expression can be converted to int or bool

    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::ConditionalExpression);
    if let Ok((child_node, pos)) = p_logical_or_expression(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::QuestionMark) {
            // first judge logical_or_expression is IConstant.
            if sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Int),
            ) || sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Bool),
            ) || sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Long),
            ) || sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Signed),
            ) || sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Unsigned),
            ) || sema::judge_type_same(
                &child_node.type_exp,
                &TypeExpression::new_val(BaseType::Char),
            ) {

            } else {
                return Err(format!(
                    "Conditional Expression doesn't have logical expression"
                ));
            }
            cur_node.child.push(child_node);
            let pos = pos + 1;
            let (child_node, pos) = p_expression(toks, pos)?;
            let l_type = child_node.type_exp.clone();

            cur_node.child.push(child_node);
            check_tok(pos, &toks, &lexer::TokType::Colon)?;
            let pos = pos + 1;
            let (child_node, pos) = p_conditional_expression(toks, pos)?;
            let r_type = child_node.type_exp.clone();

            // TODO: actually they don't need to have same type, but need to be able to convert to the same type.
            //       which is the type of the left side of assignment.
            if sema::judge_type_same(&l_type, &r_type) == false {
                return Err(format!(
                    "Two option expression in Teneray Expression has different type"
                ));
            }
            cur_node.type_exp = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        }
    } else {
        return Err(format!("Error parse logical_or_expressiong"));
    }
}

// assignment_expression
// 	: conditional_expression
// 	| unary_expression assignment_operator assignment_expression
// 	;

fn p_assignment_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::AssignmentExpression);
    if let Ok((child_node1, pos1)) = p_unary_expression(toks, pos) {
        if let Ok((child_node2, pos2)) = p_assignment_operator(toks, pos1) {
            if let Ok((child_node3, pos3)) = p_assignment_expression(toks, pos2) {
                let l_type = child_node1.type_exp.clone();
                let r_type = child_node3.type_exp.clone();
                cur_node.child.push(child_node1);
                cur_node.child.push(child_node2);
                cur_node.child.push(child_node3);
                let res_type = sema::implicit_type_cast(&l_type, &r_type)?;
                cur_node.type_exp = res_type.clone();
                return Ok((cur_node, pos3));
            } else {
                let (child_node, pos) = p_conditional_expression(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            }
        } else {
            let (child_node, pos) = p_conditional_expression(toks, pos)?;
            cur_node.type_exp = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        }
    } else {
        let (child_node, pos) = p_conditional_expression(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
}

// assignment_operator
// 	: '='
// 	| MulAssign
// 	| DivAssign
// 	| ModAssign
// 	| AddAssign
// 	| SubAssign
// 	| LeftAssign
// 	| RightAssign
// 	| AndAssign
// 	| XorAssign
// 	| OrAssign
// 	;
fn p_assignment_operator(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::Assign
        | lexer::TokType::MulAssign
        | lexer::TokType::DivAssign
        | lexer::TokType::ModAssign
        | lexer::TokType::AddAssign
        | lexer::TokType::SubAssign
        | lexer::TokType::LeftAssign
        | lexer::TokType::RightAssign
        | lexer::TokType::AndAssign
        | lexer::TokType::XorAssign
        | lexer::TokType::OrAssign => {
            return Ok((
                ParseNode::new(NodeType::AssignmentOperator(toks[pos].clone())),
                pos + 1,
            ));
        }
        _ => {
            return Err(error_handler("Assignment operator", &toks[pos], pos));
        }
    }
}

// expression
// 	: assignment_expression
// 	| expression ',' assignment_expression
// 	;
//  -> assignment_expression { ',' assignment_expression }
fn p_expression(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::Expression);

    let (child_node, pos) = p_assignment_expression(toks, pos)?; // if error, then out
    let pre_type = child_node.type_exp.clone();
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut inc = 0;
    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_assignment_expression(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                inc += 1;
                // pick the right most assignment_expression's type as its type.
                // if a = 1,b = 4.0, c = 5, then type should be type(c=5) = int,
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                pos = tmp_pos;
            }
            Err(_) => {
                break;
            }
        }
    }
    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}

// constant_expression
// 	: conditional_expression	/* with constraints */
// 	;
fn p_constant_expression(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::ConstantExpression);

    let (child_node, pos) = p_conditional_expression(toks, pos)?;
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);

    return Ok((cur_node, pos));
}
// declaration
// 	: declaration_specifiers ';'
// 	| declaration_specifiers init_declarator_list ';'
// 	| static_assert_declaration
// 	;
fn p_declaration(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Declaration);
    if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
            cur_node.type_exp = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            return Ok((cur_node, pos + 1));
        } else {
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);

            let (child_node, pos) = p_init_declarator_list(toks, pos)?;
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);

            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                return Err(error_handler(";", &toks[pos], pos));
            }
        }
    } else if let Ok((child_node, pos)) = p_static_assert_declaration(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Can't parse declaration"));
    }
}

// declaration_specifiers
// 	: storage_class_specifier declaration_specifiers
// 	| storage_class_specifier
// 	| type_specifier declaration_specifiers
// 	| type_specifier
// 	| type_qualifier declaration_specifiers
// 	| type_qualifier
// 	| function_specifier declaration_specifiers
// 	| function_specifier
// 	| alignment_specifier declaration_specifiers
// 	| alignment_specifier
// 	;
fn p_declaration_specifiers(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::DeclarationSpecifiers);

    if let Ok((child_node, pos)) = p_storage_class_specifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_type_specifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_type_qualifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_function_specifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_alignment_specifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_declaration_specifiers(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else {
        return Err(format!("Can't parse declaration_specifiers"));
    }
}

// init_declarator_list
// 	: init_declarator
// 	| init_declarator_list ',' init_declarator
// 	;
//  -> init_declarator { ',' init_declarator }

fn p_init_declarator_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::InitDeclaratorList);

    let (child_node, pos) = p_init_declarator(toks, pos)?; // if error, then out
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_init_declarator(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                inc += 1;

                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp_pos
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }

    return Ok((cur_node, pos));
}

// init_declarator
// 	: declarator '=' initializer
// 	| declarator
// 	;
fn p_init_declarator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::InitDeclarator);

    if let Ok((child_node, pos)) = p_declarator(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Assign) {
            let pos = pos + 1;
            let (child_node, pos) = p_initializer(toks, pos)?;

            if sema::judge_type_same(&pre_type, &child_node.type_exp) {
                // ok
            } else {
                return Err(format!("init_declarator, can not assign"));
            }

            cur_node.type_exp = pre_type;
            cur_node.child.push(child_node);

            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else {
        return Err(format!("Can't parse init_declarator"));
    }
}

// storage_class_specifier
// 	: TYPEDEF	/* identifiers must be flagged as TypedefName */
// 	| EXTERN
// 	| STATIC
// 	| ThreadLocal
// 	| AUTO
// 	| REGISTER
// 	;
fn p_storage_class_specifier(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::TYPEDEF => {
            return Err(format!("Typedef is not supported in crust now"));
        }
        lexer::TokType::EXTERN => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Extern);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::STATIC => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Static);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::ThreadLocal => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::ThreadLocal);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::AUTO => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Auto);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::REGISTER => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Register);
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("storage_class_specifier", &toks[pos], pos));
        }
    }
}

// type_specifier
// 	: VOID
// 	| CHAR
// 	| SHORT
// 	| INT
// 	| LONG
// 	| FLOAT
// 	| DOUBLE
// 	| SIGNED
// 	| UNSIGNED
// 	| BOOL
// 	| COMPLEX
// 	| IMAGINARY	  	/* non-mandated extension */
// 	| atomic_type_specifier
// 	| struct_or_union_specifier
// 	| enum_specifier
// 	| TypedefName		/* after it has been defined as such */
// 	;
fn p_type_specifier(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    match &toks[pos] {
        lexer::TokType::VOID => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Void);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::CHAR => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Char);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::SHORT => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Short);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::INT => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Int);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::LONG => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Long);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::FLOAT => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Float);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::DOUBLE => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Double);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::SIGNED => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Signed);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::UNSIGNED => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Unsigned);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::BOOL => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Bool);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::COMPLEX => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Complex);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::IMAGINARY => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Imaginary);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::TypedefName => {
            // XXX: now can not handle typedef
            return Err(format!("Typedef is not supported in crust now"));
            // let cur_node = ParseNode::new(NodeType::TypeSpecifier(Some(toks[pos].clone())));
            // cur_node.type_exp = TypeExpression::new_val(BaseType::Typedef);
            // return Ok((cur_node, pos + 1));
        }
        _ => {
            let mut cur_node = ParseNode::new(NodeType::TypeSpecifier(None));
            if let Ok((child_node, pos)) = p_atomic_type_specifier(toks, pos) {
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else if let Ok((child_node, pos)) = p_struct_or_union_specifier(toks, pos) {
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else if let Ok((child_node, pos)) = p_enum_specifier(toks, pos) {
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                return Ok((cur_node, pos));
            } else {
                return Err(format!("Error parse type specifier"));
            }
        }
    }
}

// struct_or_union_specifier
// 	: struct_or_union '{' struct_declaration_list '}'
// 	| struct_or_union IDENTIFIER '{' struct_declaration_list '}'
// 	| struct_or_union IDENTIFIER
// 	;
fn p_struct_or_union_specifier(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::StructOrUnionSpecifier);
    let (child_node, pos) = p_struct_or_union(toks, pos)?;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    if let Ok((c, pos)) = p_identifier(toks, pos) {
        cur_node.type_exp.child.push(c.type_exp.clone());
        cur_node.child.push(c);
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LBrace) {
            let pos = pos + 1;

            let (child_node, pos) = p_struct_declaration_list(toks, pos)?;
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            check_tok(pos, &toks, &lexer::TokType::RBrace)?;
            let pos = pos + 1;
            return Ok((cur_node, pos));
        } else {
            return Ok((cur_node, pos));
        }
    } else {
        check_tok(pos, &toks, &lexer::TokType::LBrace)?;
        let pos = pos + 1;

        let (c, pos) = p_struct_declaration_list(toks, pos)?;
        cur_node.type_exp.child.push(c.type_exp.clone());
        cur_node.child.push(c);

        check_tok(pos, &toks, &lexer::TokType::RParen)?;
        let pos = pos + 1;

        return Ok((cur_node, pos));
    }
}

// struct_or_union
// 	: STRUCT
// 	| UNION
// 	;
fn p_struct_or_union(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::STRUCT => {
            let mut cur_node = ParseNode::new(NodeType::StructOrUnion(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Struct);

            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::UNION => {
            let mut cur_node = ParseNode::new(NodeType::StructOrUnion(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Union);

            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("struct or union", &toks[pos], pos));
        }
    }
}
// struct_declaration_list
// 	: struct_declaration
// 	| struct_declaration_list struct_declaration
// 	;
//  -> struct_declaration { struct_declaration }
fn p_struct_declaration_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::StructDeclarationList);

    let (child_node, pos) = p_struct_declaration(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_struct_declaration(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }
    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}
// struct_declaration
// 	: specifier_qualifier_list ';'	/* for anonymous struct/union */
// 	| specifier_qualifier_list struct_declarator_list ';'
// 	| static_assert_declaration
// 	;
fn p_struct_declaration(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::StructDeclaration);
    if let Ok((child_node, pos)) = p_specifier_qualifier_list(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);

        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
            let pos = pos + 1;
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }

        let (child_node, pos) = p_struct_declarator_list(toks, pos)?;
        cur_node.type_exp.child.push(pre_type);
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);

        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
            let pos = pos + 1;
            return Ok((cur_node, pos));
        } else {
            return Err(error_handler(";", &toks[pos], pos));
        }
    } else if let Ok((child_node, pos)) = p_static_assert_declaration(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse struct declaration"));
    }
}
// specifier_qualifier_list
// 	: type_specifier specifier_qualifier_list
// 	| type_specifier
// 	| type_qualifier specifier_qualifier_list
// 	| type_qualifier
// 	;
fn p_specifier_qualifier_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::SpecifierQualifier);
    if let Ok((child_node, pos)) = p_type_specifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_specifier_qualifier_list(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_type_qualifier(toks, pos) {
        let pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_specifier_qualifier_list(toks, pos) {
            cur_node.type_exp.child.push(pre_type);
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    } else {
        return Err(format!("Error parse specifier_qualifier_list"));
    }
}
// struct_declarator_list
// 	: struct_declarator
// 	| struct_declarator_list ',' struct_declarator
// 	;
//  -> struct_declarator { ',' struct_declarator }
fn p_struct_declarator_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::StructDeclaratorList);

    let (child_node, pos) = p_struct_declarator(toks, pos)?; // if error, then out
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_struct_declarator(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                inc += 1;
                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp_pos
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}
// struct_declarator
// 	: ':' constant_expression
// 	| declarator ':' constant_expression
// 	| declarator
// 	;
fn p_struct_declarator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::StructDeclarator);
    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Colon) {
        let pos = pos + 1;
        let (child_node, pos) = p_constant_expression(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        let (child_node, pos) = p_declarator(toks, pos)?;
        let pre_type = child_node.type_exp.clone();

        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Colon) {
            let (child_node, pos) = p_constant_expression(toks, pos)?;
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            cur_node.type_exp = pre_type;
            return Ok((cur_node, pos));
        }
    }
}

// enum_specifier
// 	: ENUM '{' enumerator_list '}'
// 	| ENUM '{' enumerator_list ',' '}'
// 	| ENUM IDENTIFIER '{' enumerator_list '}'
// 	| ENUM IDENTIFIER '{' enumerator_list ',' '}'
// 	| ENUM IDENTIFIER
// 	;
// TODO: Add type system
fn p_enum_specifier(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    check_tok(pos, &toks, &lexer::TokType::ENUM)?;
    let pos = pos + 1;
    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LBrace) {
        let mut cur_node = ParseNode::new(NodeType::EnumSpecifier(None));
        let (child_node, pos) = p_enumerator_list(toks, pos)?;
        cur_node.child.push(child_node);

        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBrace) {
            let pos = pos + 1;
            return Ok((cur_node, pos));
        }

        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBrace) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                return Err(error_handler("}", &toks[pos], pos));
            }
        } else {
            return Err(error_handler("}", &toks[pos], pos));
        }
    } else {
        match &toks[pos] {
            lexer::TokType::IDENTIFIER(name) => {
                let mut cur_node = ParseNode::new(NodeType::EnumSpecifier(Some(name.to_string())));
                let pos = pos + 1;
                if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LBrace) {
                    let (child_node, pos) = p_enumerator_list(toks, pos)?;
                    cur_node.child.push(child_node);
                    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBrace) {
                        let pos = pos + 1;
                        return Ok((cur_node, pos));
                    }

                    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
                        let pos = pos + 1;
                        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBrace) {
                            let pos = pos + 1;
                            return Ok((cur_node, pos));
                        } else {
                            return Err(error_handler("}", &toks[pos], pos));
                        }
                    } else {
                        return Err(error_handler("}", &toks[pos], pos));
                    }
                } else {
                    return Err(error_handler("}", &toks[pos], pos));
                }
            }
            _ => {
                return Err(error_handler("`{` or identifier", &toks[pos], pos));
            }
        }
    }
}

// enumerator_list
// 	: enumerator
// 	| enumerator_list ',' enumerator
// 	;
//  -> enumerator { ',' enumerator }
fn p_enumerator_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::EnumeratorList);
    let (child_node, pos) = p_enumerator(toks, pos)?; // if error, then out
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_enumerator(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp_pos
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }
    return Ok((cur_node, pos));
}

// enumerator	/* identifiers must be flagged as EnumerationConstant */
// 	: enumeration_constant '=' constant_expression
// 	| enumeration_constant
// 	;
fn p_enumerator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Enumerator);
    let (child_node, pos) = p_enumeration_constant(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    cur_node.child.push(child_node);

    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Assign) {
        cur_node.type_exp.child.push(pre_type);
        let pos = pos + 1;
        let (child_node, pos) = p_constant_expression(toks, pos)?;
        // cause enum is guaranted to be enough to hold `int`, so ignore `char`
        if sema::judge_type_same(
            &child_node.type_exp,
            &TypeExpression::new_val(BaseType::Int),
        ) || sema::judge_type_same(
            &child_node.type_exp,
            &TypeExpression::new_val(BaseType::Bool),
        ) || sema::judge_type_same(
            &child_node.type_exp,
            &TypeExpression::new_val(BaseType::Long),
        ) || sema::judge_type_same(
            &child_node.type_exp,
            &TypeExpression::new_val(BaseType::Signed),
        ) || sema::judge_type_same(
            &child_node.type_exp,
            &TypeExpression::new_val(BaseType::Unsigned),
        ) {
            // ok
        } else {
            return Err(format!("enumeration_constant can only assign to int"));
        }

        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        cur_node.type_exp = pre_type;
        return Ok((cur_node, pos));
    }
}

// atomic_type_specifier
// 	: ATOMIC '(' type_name ')'
// 	;
fn p_atomic_type_specifier(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::AtomicTypeSpecifier);

    check_tok(pos, &toks, &lexer::TokType::ATOMIC)?;
    let pos = pos + 1;
    cur_node.type_exp = TypeExpression::new_val(BaseType::Atomic);

    check_tok(pos, &toks, &lexer::TokType::LParen)?;
    let pos = pos + 1;

    let (child_node, pos) = p_type_name(toks, pos)?;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    check_tok(pos, &toks, &lexer::TokType::RParen)?;
    let pos = pos + 1;

    return Ok((cur_node, pos));
}
// type_qualifier
// 	: CONST
// 	| RESTRICT
// 	| VOLATILE
// 	| ATOMIC
// 	;
fn p_type_qualifier(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::CONST => {
            let mut cur_node = ParseNode::new(NodeType::TypeQualifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Const);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::RESTRICT => {
            let mut cur_node = ParseNode::new(NodeType::TypeQualifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Restrict);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::VOLATILE => {
            let mut cur_node = ParseNode::new(NodeType::TypeQualifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Volatile);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::ATOMIC => {
            let mut cur_node = ParseNode::new(NodeType::TypeQualifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Atomic);
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler(
                "[const, restricted, volatile, atomic]",
                &toks[pos],
                pos,
            ));
        }
    }
}
// function_specifier
// 	: INLINE
// 	| NORETURN
// 	;
fn p_function_specifier(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::INLINE => {
            let mut cur_node = ParseNode::new(NodeType::FunctionSpecifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Inline);
            return Ok((cur_node, pos + 1));
        }
        lexer::TokType::NORETURN => {
            let mut cur_node = ParseNode::new(NodeType::FunctionSpecifier(toks[pos].clone()));
            cur_node.type_exp = TypeExpression::new_val(BaseType::Noreturn);
            return Ok((cur_node, pos + 1));
        }
        _ => {
            return Err(error_handler("[inline, noreturn]", &toks[pos], pos));
        }
    }
}
// alignment_specifier
// 	: ALIGNAS '(' type_name ')'
// 	| ALIGNAS '(' constant_expression ')'
// 	;
// XXX: now just return type non expression
fn p_alignment_specifier(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    check_tok(pos, &toks, &lexer::TokType::ALIGNAS)?;
    let pos = pos + 1;

    check_tok(pos, &toks, &lexer::TokType::LParen)?;
    let pos = pos + 1;

    let mut cur_node = ParseNode::new(NodeType::AlignmentSpecifier);
    let mut pos = pos;
    if let Ok((child_node, tmp_pos)) = p_type_name(toks, pos) {
        cur_node.child.push(child_node);
        pos = tmp_pos;
    } else if let Ok((child_node, tmp_pos)) = p_constant_expression(toks, pos) {
        cur_node.child.push(child_node);
        pos = tmp_pos;
    } else {
        return Err(format!("Error parse alignment_specifier"));
    }

    check_tok(pos, &toks, &lexer::TokType::RParen)?;
    let pos = pos + 1;
    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
    return Ok((cur_node, pos));
}
// declarator
// 	: pointer direct_declarator
// 	| direct_declarator
// 	;
fn p_declarator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Declarator);
    if let Ok((child_node, pos)) = p_direct_declarator(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_pointer(toks, pos) {
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        let (child_node, pos) = p_direct_declarator(toks, pos)?;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse declarator"));
    }
}

// direct_declarator
// 	: IDENTIFIER
// 	| '(' declarator ')'
// 	| direct_declarator '[' ']'
// 	| direct_declarator '[' '*' ']'
// 	| direct_declarator '[' STATIC type_qualifier_list assignment_expression ']'
// 	| direct_declarator '[' STATIC assignment_expression ']'
// 	| direct_declarator '[' type_qualifier_list '*' ']'
// 	| direct_declarator '[' type_qualifier_list STATIC assignment_expression ']'
// 	| direct_declarator '[' type_qualifier_list assignment_expression ']'
// 	| direct_declarator '[' type_qualifier_list ']'
// 	| direct_declarator '[' assignment_expression ']'
// 	| direct_declarator '(' parameter_type_list ')'
// 	| direct_declarator '(' ')'
// 	| direct_declarator '(' identifier_list ')'
// 	;
//  EBNF ->
// (IDENTIFIER|'(' declarator ')')  [direct_declarator_post_list]
/// I combine all the postfix together in one ParseNode
/// so if this root node has two child, then it has postfix,
/// otherwise just IDENTIFIER or '(' declarator ')'
fn p_direct_declarator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::DirectDeclarator);
    let mut pos = pos;

    let mut pre_type;

    if let Ok((child_node, tmp_pos)) = p_identifier(toks, pos) {
        pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        pos = tmp_pos;
    } else if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LParen) {
        let tmp_pos = pos + 1;
        let (child_node, tmp_pos) = p_declarator(toks, tmp_pos)?;
        pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        pos = tmp_pos;
    } else {
        return Err(format!("Error parse direct_declarator"));
    }

    if let Ok((child_node, pos)) = p_direct_declarator_post_list(toks, pos) {
        cur_node.type_exp.child.push(pre_type);
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        cur_node.type_exp = pre_type;
        return Ok((cur_node, pos));
    }
}

// direct_declarator_post_list
// : direct_declarator_post { direct_declarator_post }
fn p_direct_declarator_post_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::DirectDeclaratorPostList);
    let (child_node, pos) = p_direct_declarator_post(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_direct_declarator_post(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}

// direct_declarator_post
// 	| '(' parameter_type_list ')'
// 	| '(' ')'
// 	| '(' identifier_list ')'
// 	| '[' ']'
// 	| '[' assignment_expression ']'
//  FIXME: should add below situations support
// 	| '[' '*' ']'
// 	| '[' STATIC type_qualifier_list assignment_expression ']'
// 	| '[' STATIC assignment_expression ']'
// 	| '[' type_qualifier_list '*' ']'
// 	| '[' type_qualifier_list STATIC assignment_expression ']'
// 	| '[' type_qualifier_list assignment_expression ']'
// 	| '[' type_qualifier_list ']'
fn p_direct_declarator_post(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::LParen => {
            let mut cur_node = ParseNode::new(NodeType::DirectDeclaratorPost(toks[pos].clone()));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RParen) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else if let Ok((child_node, pos)) = p_parameter_type_list(toks, pos) {
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                check_tok(pos, &toks, &lexer::TokType::RParen)?;
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                let (child_node, pos) = p_identifier_list(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                check_tok(pos, &toks, &lexer::TokType::RParen)?;
                let pos = pos + 1;
                return Ok((cur_node, pos));
            }
        }
        lexer::TokType::LBracket => {
            let mut cur_node = ParseNode::new(NodeType::DirectDeclaratorPost(toks[pos].clone()));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBracket) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                let (child_node, pos) = p_assignment_expression(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                check_tok(pos, &toks, &lexer::TokType::RBracket)?;
                let pos = pos + 1;
                return Ok((cur_node, pos));
            }
        }
        _ => {
            return Err(error_handler("[ or (", &toks[pos], pos));
        }
    }
}
// pointer
// 	: '*' type_qualifier_list pointer
// 	| '*' type_qualifier_list
// 	| '*' pointer
// 	| '*'
// 	;
fn p_pointer(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Pointer);
    check_tok(pos, &toks, &lexer::TokType::Multi)?;
    cur_node.type_exp = TypeExpression::new_val(BaseType::Pointer);
    let pos = pos + 1;
    if let Ok((child_node, pos)) = p_type_qualifier_list(toks, pos) {
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        if let Ok((child_node, pos)) = p_pointer(toks, pos) {
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_pointer(toks, pos) {
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Ok((cur_node, pos));
    }
}

// type_qualifier_list
// 	: type_qualifier
// 	| type_qualifier_list type_qualifier
// 	;
//  -> type_qualifier { type_qualifier }
fn p_type_qualifier_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::TypeQualifierList);
    let (child_node, pos) = p_type_qualifier(toks, pos)?;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_type_qualifier(toks, pos) {
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }
    return Ok((cur_node, pos));
}
// parameter_type_list
// 	: parameter_list ',' ELLIPSIS
// 	| parameter_list
// 	;
fn p_parameter_type_list(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::ParameterTypeList(false)); // no extra variable
    let (child_node, pos) = p_parameter_list(toks, pos)?;
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
        let pos = pos + 1;
        check_tok(pos, &toks, &lexer::TokType::ELLIPSIS)?;
        cur_node.entry = NodeType::ParameterTypeList(true);
        // XXX: VaList in node.type_exp.val
        cur_node.type_exp.val.push(BaseType::VaList);
        return Ok((cur_node, pos));
    } else {
        return Ok((cur_node, pos));
    }
}

// parameter_list
// 	: parameter_declaration
// 	| parameter_list ',' parameter_declaration
// 	;
//  -> parameter_declaration { ',' parameter_declaration }
fn p_parameter_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::ParameterList);
    let (child_node, pos) = p_parameter_declaration(toks, pos)?; // if error, then out
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_parameter_declaration(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp_pos
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }
    return Ok((cur_node, pos));
}

// parameter_declaration
// 	: declaration_specifiers declarator
// 	| declaration_specifiers abstract_declarator
// 	| declaration_specifiers
// 	;
fn p_parameter_declaration(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::ParameterDeclaration);
    let (c, pos) = p_declaration_specifiers(toks, pos)?;
    let declaration_specifiers_type = c.type_exp.clone();

    cur_node.child.push(c);
    if let Ok((c, pos)) = p_declarator(toks, pos) {
        cur_node.type_exp.child.push(declaration_specifiers_type);
        cur_node.type_exp.child.push(c.type_exp.clone());
        cur_node.child.push(c);
        return Ok((cur_node, pos));
    } else if let Ok((c, pos)) = p_abstract_declarator(toks, pos) {
        cur_node.type_exp.child.push(declaration_specifiers_type);
        cur_node.type_exp.child.push(c.type_exp.clone());
        cur_node.child.push(c);
        return Ok((cur_node, pos));
    } else {
        cur_node.type_exp = declaration_specifiers_type;
        return Ok((cur_node, pos));
    }
}

// identifier_list
// 	: IDENTIFIER
// 	| identifier_list ',' IDENTIFIER
// 	;
//  -> IDENTIFIER { ',' IDENTIFIER }
fn p_identifier_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node: ParseNode = ParseNode::new(NodeType::IdentifierList);
    let (child_node, pos) = p_identifier(toks, pos)?; // if error, then out
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }

        match p_identifier(toks, pos) {
            Ok((child_node, tmp_pos)) => {
                inc += 1;
                cur_node.type_exp.child.push(child_node.type_exp.clone());
                cur_node.child.push(child_node);
                pos = tmp_pos
            }
            Err(_) => {
                pos = pos - 1;
                break;
            }
        }
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}
// type_name
// 	: specifier_qualifier_list abstract_declarator
// 	| specifier_qualifier_list
// 	;
fn p_type_name(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::TypeName);
    let (child_node, pos) = p_specifier_qualifier_list(toks, pos)?;
    let specifier_qualifier_list_type = child_node.type_exp.clone();
    cur_node.child.push(child_node);

    if let Ok((child_node, pos)) = p_abstract_declarator(toks, pos) {
        cur_node.type_exp.child.push(specifier_qualifier_list_type);
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        cur_node.type_exp = specifier_qualifier_list_type;
        return Ok((cur_node, pos));
    }
}

// abstract_declarator
// 	: pointer direct_abstract_declarator
// 	| pointer
// 	| direct_abstract_declarator
// 	;
fn p_abstract_declarator(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut cur_node = ParseNode::new(NodeType::AbstractDeclarator);

    if let Ok((child_node, pos)) = p_pointer(toks, pos) {
        cur_node.child.push(child_node);
        cur_node.type_exp = TypeExpression::new_val(BaseType::Pointer);
        if let Ok((child_node, pos)) = p_direct_abstract_declarator(toks, pos) {
            cur_node.type_exp.child.push(child_node.type_exp.clone());
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        } else {
            return Ok((cur_node, pos));
        }
    } else if let Ok((child_node, pos)) = p_direct_abstract_declarator(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse abstract_declarator"));
    }
}

// direct_abstract_declarator
// 	: '(' abstract_declarator ')'
// 	| '[' ']'
// 	| '[' '*' ']'
// 	| '[' STATIC type_qualifier_list assignment_expression ']'
// 	| '[' STATIC assignment_expression ']'
// 	| '[' type_qualifier_list STATIC assignment_expression ']'
// 	| '[' type_qualifier_list assignment_expression ']'
// 	| '[' type_qualifier_list ']'
// 	| '[' assignment_expression ']'
// 	| '(' ')'
// 	| '(' parameter_type_list ')'
// 	| direct_abstract_declarator '[' ']'
// 	| direct_abstract_declarator '[' '*' ']'
// 	| direct_abstract_declarator '[' STATIC type_qualifier_list assignment_expression ']'
// 	| direct_abstract_declarator '[' STATIC assignment_expression ']'
// 	| direct_abstract_declarator '[' type_qualifier_list assignment_expression ']'
// 	| direct_abstract_declarator '[' type_qualifier_list STATIC assignment_expression ']'
// 	| direct_abstract_declarator '[' type_qualifier_list ']'
// 	| direct_abstract_declarator '[' assignment_expression ']'
// 	| direct_abstract_declarator '(' ')'
// 	| direct_abstract_declarator '(' parameter_type_list ')'
// 	;
//  EBNF ->
// direct_abstract_declarator_block { direct_abstract_declarator_block }

fn p_direct_abstract_declarator(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::DirectAbstractDeclarator);
    let (child_node, pos) = p_direct_abstract_declarator_block(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_direct_abstract_declarator_block(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }

    return Ok((cur_node, pos));
}
// direct_abstract_declarator_block
// 	: '(' abstract_declarator ')'
// 	| '(' ')'
// 	| '(' parameter_type_list ')'
// 	| '[' ']'
// 	| '[' assignment_expression ']'
//  FIXME: should add below situations support.
// 	| '[' '*' ']'
// 	| '[' STATIC type_qualifier_list assignment_expression ']'
// 	| '[' STATIC assignment_expression ']'
// 	| '[' type_qualifier_list STATIC assignment_expression ']'
// 	| '[' type_qualifier_list assignment_expression ']'
// 	| '[' type_qualifier_list ']'
fn p_direct_abstract_declarator_block(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::LParen => {
            let mut cur_node =
                ParseNode::new(NodeType::DirectAbstractDeclaratorBlock(toks[pos].clone()));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RParen) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                if let Ok((child_node, pos)) = p_abstract_declarator(toks, pos) {
                    cur_node.type_exp = child_node.type_exp.clone();
                    cur_node.child.push(child_node);
                    check_tok(pos, &toks, &lexer::TokType::RParen)?;
                    let pos = pos + 1;
                    return Ok((cur_node, pos));
                } else {
                    let (child_node, pos) = p_parameter_type_list(toks, pos)?;
                    cur_node.type_exp = child_node.type_exp.clone();
                    cur_node.child.push(child_node);
                    check_tok(pos, &toks, &lexer::TokType::RParen)?;
                    let pos = pos + 1;
                    return Ok((cur_node, pos));
                }
            }
        }
        lexer::TokType::LBracket => {
            let mut cur_node =
                ParseNode::new(NodeType::DirectAbstractDeclaratorBlock(toks[pos].clone()));
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RBracket) {
                let pos = pos + 1;
                return Ok((cur_node, pos));
            } else {
                let (child_node, pos) = p_assignment_expression(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                check_tok(pos, &toks, &lexer::TokType::RBracket)?;
                let pos = pos + 1;
                return Ok((cur_node, pos));
            }
        }
        _ => {
            return Err(error_handler("( or [", &toks[pos], pos));
        }
    }
}

// initializer
// 	: '{' initializer_list '}'
// 	| '{' initializer_list ',' '}'
// 	| assignment_expression
// 	;
fn p_initializer(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Initializer);

    if let Ok((child_node, pos)) = p_assignment_expression(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        check_tok(pos, &toks, &lexer::TokType::LBrace)?;
        let pos = pos + 1;

        let (child_node, pos) = p_initializer_list(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);

        if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::RBrace)?;
            return Ok((cur_node, pos));
        }
        check_tok(pos, &toks, &lexer::TokType::RBrace)?;
        let pos = pos + 1;
        return Ok((cur_node, pos));
    }
}
// initializer_list
// 	: designation initializer
// 	| initializer
// 	| initializer_list ',' designation initializer
// 	| initializer_list ',' initializer
// 	;
// -> pre {',' pre}
// XXX: designation initializer should get type(initializer) as its type
//      but need to add judge function to judge whether it's ok to assign
fn p_initializer_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    let mut pre_type;

    let mut inc = 0;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::InitializerList);
    let mut pos = pos;
    if let Ok((child_node, tmp_pos)) = p_initializer(toks, pos) {
        pos = tmp_pos;
        pre_type = child_node.type_exp.clone();
        cur_node.child.push(child_node);
    } else if let Ok((child_node, tmp_pos)) = p_designation(toks, pos) {
        pos = tmp_pos;
        cur_node.child.push(child_node);
        let (child_node, tmp_pos) = p_initializer(toks, pos)?;
        pre_type = child_node.type_exp.clone();
        pos = tmp_pos;
        cur_node.child.push(child_node);
    } else {
        return Err(format!("Error parse initializer_list"));
    }
    cur_node.type_exp.child.push(pre_type.clone());

    loop {
        if let Err(_) = check_tok(pos, &toks, &lexer::TokType::Comma) {
            break;
        } else {
            pos = pos + 1;
        }
        inc = inc + 1;
        if let Ok((child_node, tmp_pos)) = p_initializer(toks, pos) {
            pre_type = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            pos = tmp_pos;
        } else if let Ok((child_node, tmp_pos)) = p_designation(toks, pos) {
            pos = tmp_pos;
            cur_node.child.push(child_node);
            let (child_node, tmp_pos) = p_initializer(toks, pos)?;
            pre_type = child_node.type_exp.clone();
            cur_node.child.push(child_node);
            pos = tmp_pos;
        } else {
            pos = pos - 1;
            break;
        }
        cur_node.type_exp.child.push(pre_type.clone());
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }

    return Ok((cur_node, pos));
}

// designation
// 	: designator_list '='
// 	;
fn p_designation(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Designation);
    let (child_node, pos) = p_designator_list(toks, pos)?;
    cur_node.type_exp = child_node.type_exp.clone();
    cur_node.child.push(child_node);
    check_tok(pos, &toks, &lexer::TokType::Assign)?;
    let pos = pos + 1;
    return Ok((cur_node, pos));
}
// designator_list
// 	: designator
// 	| designator_list designator
// 	;
//  -> designator { designator }
fn p_designator_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::DesignatorList);
    let (child_node, pos) = p_designator(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_designator(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }

    return Ok((cur_node, pos));
}

// designator
// 	: '[' constant_expression ']'
// 	| '.' IDENTIFIER
// 	;
fn p_designator(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Designator);
    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::LBracket) {
        let pos = pos + 1;
        let (child_node, pos) = p_constant_expression(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        check_tok(pos, &toks, &lexer::TokType::RBracket)?;
        let pos = pos + 1;
        return Ok((cur_node, pos));
    } else {
        check_tok(pos, &toks, &lexer::TokType::Dot)?;
        let pos = pos + 1;
        let (child_node, pos) = p_identifier(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
}

// static_assert_declaration
// 	: StaticAssert '(' constant_expression ',' StringLiteral ')' ';'
// 	;
fn p_static_assert_declaration(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    check_tok(pos, &toks, &lexer::TokType::StaticAssert)?;
    let pos = pos + 1;

    check_tok(pos, &toks, &lexer::TokType::LParen)?;
    let pos = pos + 1;
    let mut cur_node = ParseNode::new(NodeType::StaticAssertDeclaration);
    let (child_node, pos) = p_constant_expression(toks, pos)?;
    cur_node.child.push(child_node);
    check_tok(pos, &toks, &lexer::TokType::Comma)?;
    let pos = pos + 1;

    let (child_node, pos) = p_string(toks, pos)?;
    cur_node.child.push(child_node);

    check_tok(pos, &toks, &lexer::TokType::RParen)?;
    let pos = pos + 1;
    check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
    let pos = pos + 1;
    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
    return Ok((cur_node, pos));
}

// statement
// 	: labeled_statement
// 	| compound_statement
// 	| expression_statement
// 	| selection_statement
// 	| iteration_statement
// 	| jump_statement
// 	;
fn p_statement(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::Statement);
    if let Ok((child_node, pos)) = p_labeled_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_compound_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_expression_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_selection_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_iteration_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_jump_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse statement"));
    }
}
// labeled_statement
// 	: IDENTIFIER ':' statement
// 	| CASE constant_expression ':' statement
// 	| DEFAULT ':' statement
// 	;
fn p_labeled_statement(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::LabeledStatement("".to_string()));
    match &toks[pos] {
        lexer::TokType::IDENTIFIER(s) => {
            cur_node.entry = NodeType::LabeledStatement(s.to_string());
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::Colon)?;
            let pos = pos + 1;
            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            cur_node.child.push(child_node);
            return Ok((cur_node, pos));
        }
        lexer::TokType::CASE => {
            cur_node.entry = NodeType::LabeledStatement("case".to_string());
            let pos = pos + 1;
            let (child_node, pos) = p_constant_expression(toks, pos)?;
            cur_node.child.push(child_node);
            check_tok(pos, &toks, &lexer::TokType::Colon)?;
            let pos = pos + 1;
            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);
            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        lexer::TokType::DEFAULT => {
            cur_node.entry = NodeType::LabeledStatement("default".to_string());
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::Colon)?;
            let pos = pos + 1;
            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);
            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        _ => {
            return Err(error_handler("label", &toks[pos], pos));
        }
    }
}

// compound_statement
// 	: '{' '}'
// 	| '{'  block_item_list '}'
// 	;
fn p_compound_statement(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::CompoundStatement);
    check_tok(pos, &toks, &lexer::TokType::LBrace)?;
    let pos = pos + 1;
    if let Ok((child_node, pos)) = p_block_item_list(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        check_tok(pos, &toks, &lexer::TokType::RBrace)?;
        let pos = pos + 1;
        return Ok((cur_node, pos));
    } else {
        check_tok(pos, &toks, &lexer::TokType::RBrace)?;
        let pos = pos + 1;
        cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
        return Ok((cur_node, pos));
    }
}
// block_item_list
// 	: block_item
// 	| block_item_list block_item
// 	;
//  -> block_item { block_item }
fn p_block_item_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::BlockItemList);
    let (child_node, pos) = p_block_item(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_block_item(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }
    return Ok((cur_node, pos));
}

// block_item
// 	: declaration
// 	| statement
// 	;
fn p_block_item(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::BlockItem);
    if let Ok((child_node, pos)) = p_declaration(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else if let Ok((child_node, pos)) = p_statement(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        return Err(format!("Error parse block_item"));
    }
}

// expression_statement
// 	: ';'
// 	| expression ';'
// 	;
fn p_expression_statement(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::ExpressionStatement);
    if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
        let pos = pos + 1;
        cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
        return Ok((cur_node, pos));
    } else {
        let (child_node, pos) = p_expression(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
        let pos = pos + 1;
        return Ok((cur_node, pos));
    }
}

// selection_statement
// 	: IF '(' expression ')' statement ELSE statement
// 	| IF '(' expression ')' statement
// 	| SWITCH '(' expression ')' statement
// 	;
fn p_selection_statement(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::IF => {
            let mut cur_node = ParseNode::new(NodeType::SelectionStatement(toks[pos].clone()));
            let pos = pos + 1;

            check_tok(pos, &toks, &lexer::TokType::LParen)?;
            let pos = pos + 1;

            let (child_node, pos) = p_expression(toks, pos)?;
            cur_node.child.push(child_node);

            check_tok(pos, &toks, &lexer::TokType::RParen)?;
            let pos = pos + 1;

            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);

            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::ELSE) {
                let pos = pos + 1;
                let (child_node, pos) = p_statement(toks, pos)?;
                cur_node.child.push(child_node);
                cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                return Ok((cur_node, pos));
            } else {
                cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                return Ok((cur_node, pos));
            }
        }
        lexer::TokType::SWITCH => {
            let mut cur_node = ParseNode::new(NodeType::SelectionStatement(toks[pos].clone()));
            let pos = pos + 1;

            check_tok(pos, &toks, &lexer::TokType::LParen)?;
            let pos = pos + 1;
            let (child_node, pos) = p_expression(toks, pos)?;
            cur_node.child.push(child_node);

            check_tok(pos, &toks, &lexer::TokType::RParen)?;
            let pos = pos + 1;
            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);

            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        _ => {
            return Err(error_handler("[if, switch]", &toks[pos], pos));
        }
    }
}

// iteration_statement
// 	: WHILE '(' expression ')' statement
// 	| DO statement WHILE '(' expression ')' ';'
// 	| FOR '(' expression_statement expression_statement ')' statement
// 	| FOR '(' expression_statement expression_statement expression ')' statement
// 	| FOR '(' declaration expression_statement ')' statement
// 	| FOR '(' declaration expression_statement expression ')' statement
// 	;
fn p_iteration_statement(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    match &toks[pos] {
        lexer::TokType::WHILE => {
            // 	: WHILE '(' expression ')' statement
            let mut cur_node = ParseNode::new(NodeType::IterationStatement(toks[pos].clone()));
            let pos = pos + 1;

            check_tok(pos, &toks, &lexer::TokType::LParen)?;
            let pos = pos + 1;

            let (child_node, pos) = p_expression(toks, pos)?;
            cur_node.child.push(child_node);

            check_tok(pos, &toks, &lexer::TokType::RParen)?;
            let pos = pos + 1;

            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);

            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        lexer::TokType::DO => {
            let mut cur_node = ParseNode::new(NodeType::IterationStatement(toks[pos].clone()));
            let pos = pos + 1;
            let (child_node, pos) = p_statement(toks, pos)?;
            cur_node.child.push(child_node);

            check_tok(pos, &toks, &lexer::TokType::WHILE)?;
            let pos = pos + 1;

            check_tok(pos, &toks, &lexer::TokType::LParen)?;
            let pos = pos + 1;

            let (child_node, pos) = p_expression(toks, pos)?;
            cur_node.child.push(child_node);

            check_tok(pos, &toks, &lexer::TokType::RParen)?;
            let pos = pos + 1;

            check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
            let pos = pos + 1;

            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        lexer::TokType::FOR => {
            // 	| FOR '(' expression_statement expression_statement ')' statement
            // 	| FOR '(' expression_statement expression_statement expression ')' statement
            // 	| FOR '(' declaration expression_statement ')' statement
            // 	| FOR '(' declaration expression_statement expression ')' statement
            let mut cur_node = ParseNode::new(NodeType::IterationStatement(toks[pos].clone()));
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::LParen)?;
            let pos = pos + 1;
            if let Ok((child_node, pos)) = p_expression_statement(toks, pos) {
                cur_node.child.push(child_node);
                let (child_node, pos) = p_expression_statement(toks, pos)?;
                cur_node.child.push(child_node);
                if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RParen) {
                    // 	| FOR '(' expression_statement expression_statement ')' statement
                    let pos = pos + 1;
                    let (child_node, pos) = p_statement(toks, pos)?;
                    cur_node.child.push(child_node);
                    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                    return Ok((cur_node, pos));
                } else {
                    // 	| FOR '(' expression_statement expression_statement expression ')' statement
                    let (child_node, pos) = p_expression(toks, pos)?;
                    cur_node.child.push(child_node);

                    check_tok(pos, &toks, &lexer::TokType::RParen)?;
                    let pos = pos + 1;

                    let (child_node, pos) = p_statement(toks, pos)?;
                    cur_node.child.push(child_node);

                    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                    return Ok((cur_node, pos));
                }
            } else if let Ok((child_node, pos)) = p_declaration(toks, pos) {
                cur_node.child.push(child_node);
                let (child_node, pos) = p_expression_statement(toks, pos)?;
                cur_node.child.push(child_node);
                if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::RParen) {
                    // 	| FOR '(' declaration expression_statement ')' statement
                    let pos = pos + 1;

                    let (child_node, pos) = p_statement(toks, pos)?;
                    cur_node.child.push(child_node);
                    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                    return Ok((cur_node, pos));
                } else {
                    // 	| FOR '(' declaration expression_statement expression ')' statement
                    let (child_node, pos) = p_expression(toks, pos)?;
                    cur_node.child.push(child_node);

                    check_tok(pos, &toks, &lexer::TokType::RParen)?;
                    let pos = pos + 1;

                    let (child_node, pos) = p_statement(toks, pos)?;
                    cur_node.child.push(child_node);

                    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                    return Ok((cur_node, pos));
                }
            } else {
                return Err(format!("Error parse For"));
            }
        }
        _ => {
            return Err(error_handler("[while, do, for]", &toks[pos], pos));
        }
    }
}

// jump_statement
// 	: GOTO IDENTIFIER ';'
// 	| CONTINUE ';'
// 	| BREAK ';'
// 	| RETURN ';'
// 	| RETURN expression ';'
// 	;
fn p_jump_statement(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;

    match &toks[pos] {
        lexer::TokType::GOTO => {
            let pos = pos + 1;
            check_pos(pos, toks.len())?;
            match &toks[pos] {
                lexer::TokType::IDENTIFIER(var) => {
                    let mut cur_node = ParseNode::new(NodeType::JumpStatement(
                        "goto".to_string(),
                        Some(var.to_string()),
                    ));
                    let pos = pos + 1;
                    check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
                    let pos = pos + 1;
                    cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                    return Ok((cur_node, pos));
                }
                _ => {
                    return Err(error_handler("identifier for goto ", &toks[pos], pos));
                }
            }
        }
        lexer::TokType::CONTINUE => {
            let mut cur_node =
                ParseNode::new(NodeType::JumpStatement("continue".to_string(), None));
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
            let pos = pos + 1;
            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        lexer::TokType::BREAK => {
            let mut cur_node = ParseNode::new(NodeType::JumpStatement("break".to_string(), None));
            let pos = pos + 1;
            check_tok(pos, &toks, &lexer::TokType::Semicolon)?;
            let pos = pos + 1;
            cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
            return Ok((cur_node, pos));
        }
        lexer::TokType::RETURN => {
            let pos = pos + 1;
            if let Ok(_) = check_tok(pos, &toks, &lexer::TokType::Semicolon) {
                // return val, so the type for this statement should be type(val)
                let mut cur_node =
                    ParseNode::new(NodeType::JumpStatement("return".to_string(), None));
                let pos = pos + 1;
                cur_node.type_exp = TypeExpression::new_val(BaseType::NoneExpression);
                return Ok((cur_node, pos));
            } else {
                let mut cur_node =
                    ParseNode::new(NodeType::JumpStatement("return".to_string(), None));
                let (child_node, pos) = p_expression(toks, pos)?;
                cur_node.type_exp = child_node.type_exp.clone();
                cur_node.child.push(child_node);
                let pos = pos + 1;
                return Ok((cur_node, pos));
            }
        }
        _ => {
            return Err(error_handler(
                "[goto, continue, break, return]",
                &toks[pos],
                pos,
            ));
        }
    }
}

// external_declaration
// 	: function_definition
// 	| declaration
// 	;
fn p_external_declaration(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::ExternalDeclaration);
    if let Ok((child_node, pos)) = p_function_definition(toks, pos) {
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        let (child_node, pos) = p_declaration(toks, pos)?;
        cur_node.type_exp = child_node.type_exp.clone();
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
}

// function_definition
// 	: declaration_specifiers declarator declaration_list compound_statement
// 	| declaration_specifiers declarator compound_statement
// 	;
fn p_function_definition(
    toks: &[lexer::TokType],
    pos: usize,
) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node = ParseNode::new(NodeType::FunctionDefinition);
    cur_node.type_exp = TypeExpression::new_val(BaseType::Function);

    let (child_node, pos) = p_declaration_specifiers(toks, pos)?;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    let (child_node, pos) = p_declarator(toks, pos)?;
    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);

    if let Ok((child_node, pos)) = p_declaration_list(toks, pos) {
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);

        let (child_node, pos) = p_compound_statement(toks, pos)?;

        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    } else {
        let (child_node, pos) = p_compound_statement(toks, pos)?;

        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        return Ok((cur_node, pos));
    }
}
// declaration_list
// 	: declaration
// 	| declaration_list declaration
// 	;
//  -> declaration { declaration }
fn p_declaration_list(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::DeclarationList);
    let (child_node, pos) = p_declaration(toks, pos)?;
    let pre_type = child_node.type_exp.clone();
    let mut inc = 0;

    cur_node.type_exp.child.push(child_node.type_exp.clone());
    cur_node.child.push(child_node);
    let mut pos: usize = pos;
    while let Ok((child_node, tmp_pos)) = p_declaration(toks, pos) {
        inc += 1;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }

    if inc == 0 {
        cur_node.type_exp = pre_type;
    }

    return Ok((cur_node, pos));
}

// translation_unit
// 	: external_declaration
// 	| translation_unit external_declaration
// 	;
//  -> external_declaration { external_declaration }
fn p_translation_unit(toks: &[lexer::TokType], pos: usize) -> Result<(ParseNode, usize), String> {
    check_pos(pos, toks.len())?;
    let mut cur_node: ParseNode = ParseNode::new(NodeType::TranslationUnit);
    let mut pos: usize = pos;
    loop {
        if pos >= toks.len() {
            break;
        }
        let (child_node, tmp_pos) = p_external_declaration(toks, pos)?;
        cur_node.type_exp.child.push(child_node.type_exp.clone());
        cur_node.child.push(child_node);
        pos = tmp_pos;
    }
    return Ok((cur_node, pos));
}

pub fn parser_driver(toks: &[lexer::TokType], c_src_name: &str) -> Result<ParseNode, String> {
    let (cur_node, pos) = p_translation_unit(&toks, 0)?;
    if pos == toks.len() {
        return Ok(cur_node);
    } else {
        Err(format!(
            "Parser drive fails to parse the file {}",
            c_src_name
        ))
    }
}

pub fn parser_pretty_printer(tree: &ParseNode, depth: usize) -> String {
    let mut idt = String::new();
    for _i in 0..depth {
        idt = idt + "-";
    }
    let idt = idt;
    let title: String = match &tree.entry {
        NodeType::BinaryExpression(op) => format!(
            "\n{}type: {:?}, op: {:?} t_exp: {}:",
            idt,
            tree.entry,
            op,
            tree.type_exp.print()
        ),
        NodeType::Constant(t) => format!("\n{}type: {:?}, type: {:?} :", idt, tree.entry, t,),
        NodeType::EnumerationConstant(s) => format!(
            "\n{}type: {:?}, name: {:?}, t_exp {}",
            idt,
            tree.entry,
            s,
            tree.type_exp.print()
        ),
        NodeType::Identifier(name) => format!("\n{}type: {:?}, name: {:?}", idt, tree.entry, name),
        NodeType::STRING(val) => format!("\n{}type: {:?}, val: {}", idt, tree.entry, val),
        NodeType::PostfixExpressionPost(punc) => {
            format!("\n{}type: {:?}, punc: {:?}", idt, tree.entry, punc)
        }
        NodeType::UnaryExpression(op) => format!("\n{}type: {:?}, op: {:?} :", idt, tree.entry, op),
        NodeType::UnaryOperator(op) => format!("\n{}type: {:?}, op: {:?} :", idt, tree.entry, op),
        NodeType::AssignmentOperator(op) => {
            format!("\n{}type: {:?}, op: {:?} :", idt, tree.entry, op)
        }
        NodeType::StorageClassSpecifier(class) => {
            format!("\n{}type: {:?}, class: {:?} :", idt, tree.entry, class)
        }
        NodeType::TypeSpecifier(t) => format!("\n{}type: {:?}, type: {:?} :", idt, tree.entry, t),
        NodeType::StructOrUnion(t) => format!("\n{}type: {:?}, type: {:?} :", idt, tree.entry, t),
        NodeType::EnumSpecifier(n) => format!("\n{}type: {:?}, name: {:?} :", idt, tree.entry, n),
        NodeType::TypeQualifier(t) => format!("\n{}type: {:?}, type: {:?} :", idt, tree.entry, t),
        NodeType::FunctionSpecifier(n) => {
            format!("\n{}type: {:?}, name: {:?} :", idt, tree.entry, n)
        }
        NodeType::DirectDeclaratorPost(punc) => {
            format!("\n{}type: {:?}, punctuator: {:?} :", idt, tree.entry, punc)
        }
        NodeType::ParameterTypeList(has_var_arg_list) => format!(
            "\n{}type: {:?}, has_var_arg_list: {}, t_exp: {}",
            idt,
            tree.entry,
            has_var_arg_list,
            tree.type_exp.print()
        ),
        NodeType::DirectAbstractDeclaratorBlock(punc) => format!(
            "\n{}type: {:?}, punctuator: {:?} t_exp: {} :",
            idt,
            tree.entry,
            punc,
            tree.type_exp.print()
        ),
        NodeType::LabeledStatement(name) => format!(
            "\n{}type: {:?}, key: {:?} t_exp: {} :",
            idt,
            tree.entry,
            name,
            tree.type_exp.print()
        ),
        NodeType::SelectionStatement(name) => format!(
            "\n{}type: {:?}, key: {:?} : t_exp: {}: ",
            idt,
            tree.entry,
            name,
            tree.type_exp.print()
        ),
        NodeType::IterationStatement(name) => format!(
            "\n{}type: {:?}, key: {:?} t_exp: {}:",
            idt,
            tree.entry,
            name,
            tree.type_exp.print()
        ),
        NodeType::JumpStatement(name, label) => format!(
            "\n{}type: {:?} key: {}, label: {} t_exp: {}: ",
            idt,
            tree.entry,
            name,
            match label {
                Some(s) => s,
                None => "none",
            },
            tree.type_exp.print()
        ),
        _ =>
        // format!(""),
        {
            format!(
                "\n{}type: {:?} t_exp: {}:",
                idt,
                tree.entry,
                tree.type_exp.print()
            )
        }
    };
    let mut tree_s = "".to_string();
    for it in tree.child.iter() {
        tree_s += &parser_pretty_printer(it, depth + 1);
    }
    return format!("{}{}", title, tree_s);
}
