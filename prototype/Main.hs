{-# LANGUAGE TemplateHaskell #-}
import           Data.List (find, foldl)
import           Types
import           Verifier

-- Body 1 of sir/std.sexp is dup.
dupBody :: Body
dupBody =
  Body 1 [
  Block [Bind 0, Load Local 0, Load Local 0] End
  ]

formatRow :: Row -> String
formatRow (Prefix row)    = show row
formatRow (Suffix ty row) = unwords [show ty, formatRow row]

formatNames :: Show a => [a] -> String
formatNames names = "(" ++ unwords (map show names) ++ ")\n"

printEquation :: Supply -> RowVar -> Row -> [Constraint] -> IO ()
printEquation s input output equations = do
  let type_constraints =
        [unwords [" ", show a, "=", show b] | TypeEq a b <- equations]
      row_constraints =
        [unwords [" ", show a, "=", show t, show b] | RowEq a t b <- equations]
    in do
      putStrLn $ (show input) ++ " -- " ++ (show output);
      putStrLn $ "Type Constraints:\n" ++ unlines type_constraints;
      putStrLn $ "Row Constraints:\n" ++ unlines row_constraints;


main :: IO ()
main =
  let (s, input, output, equations) = constrainBody dupBody
  in printEquation s input output equations
