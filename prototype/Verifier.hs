module Verifier where

import           Data.List (foldl', mapAccumL)
import           Types

data Analysis = Analysis
  { supply      :: Supply
  , stack       :: Row
  , constraints :: [Constraint]
  }

constrainInstruction :: [TypeVar] -> Analysis -> Instruction -> Analysis
constrainInstruction locals a (Bind index) =
  let local = locals !! index in
  case stack a of
    Prefix currentPrefix ->
      let (tailRow, s) = freshRow (supply a)
       in a {supply = s,
             stack = Prefix tailRow,
             constraints = RowEq currentPrefix local tailRow : constraints a}
    Suffix ty rest ->
      a {stack = rest, constraints = TypeEq local ty : constraints a}

constrainInstruction locals a (Load kind index) =
  let local = locals !! index in
  case kind of
    Local    -> a {stack = Suffix local (stack a)}
    Captured -> error "Only local loads are supported"

constrainBody :: Body -> (Supply, RowVar, Row, [Constraint])
constrainBody (Body localCount blocks) =
  let allocate s _ = let (v, s') = freshType s in (s', v)
      (s, locals) = mapAccumL allocate emptySupply [1 .. localCount]
      (input, s') = freshRow s
      Block instructions terminator = head blocks
      result = foldl' (constrainInstruction locals)
                      (Analysis s' (Prefix input) []) instructions
   in
    case terminator of
        End -> (supply result, input, stack result,
                reverse (constraints result))
        Branch _ _ -> error "Only an End terminator is supported"
        Jump _ -> error "Only an End terminator is supported"
