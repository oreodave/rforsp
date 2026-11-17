module Verifier where

import           Data.List  (find, foldl', mapAccumL)
import           Data.Maybe (isJust)
import           Types

data Analysis = Analysis
  { supply      :: Supply
  , input       :: Row
  , stack       :: Row
  , constraints :: [Constraint]
  }


constrainInstruction :: [TypeVar] -> Analysis -> Instruction -> Analysis
constrainInstruction locals a (Bind index) =
  let local = locals !! index in
  case stack a of
    Prefix current_prefix ->
      let (s, tail_row) = freshRow (supply a)
       in a {supply = s,
             stack = Prefix tail_row,
             constraints = RowEq current_prefix local tail_row : constraints a}
    Suffix ty rest | ty == local -> a {stack = rest}
    Suffix ty rest
      | isJust $ find (\x -> x == (TypeEq ty local)) $ constraints a ->
          a {stack = rest}
    Suffix ty rest ->
      a {stack = rest, constraints = (TypeEq local ty : constraints a)}
constrainInstruction locals a (Load kind index) =
  let local = locals !! index in
  case kind of
    Local    -> a {stack = Suffix local $ stack a}
    Captured -> error "Only local loads are supported"


constrainBody :: Body -> Analysis
constrainBody (Body localCount blocks) =
  let allocate sup _ = freshType sup
      (s, locals) = mapAccumL allocate emptySupply [1 .. localCount]
      (s', input) = freshRow s
      Block instructions terminator = head blocks
      result = foldl' (constrainInstruction locals)
               (Analysis s' (Prefix input) (Prefix input) [])
               instructions
   in
    case terminator of
        End        -> result {constraints = reverse $ constraints result}
        Branch _ _ -> error "Only an End terminator is supported"
        Jump _     -> error "Only an End terminator is supported"
