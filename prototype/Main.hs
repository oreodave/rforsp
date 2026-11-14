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

dropBody :: Body
dropBody =
  Body 1 [
  Block [Bind 0] End
  ]

rotBody :: Body
rotBody =
  Body 3 [
  Block [Bind 0, Bind 1, Bind 2, Load Local 1, Load Local 0, Load Local 2] End
  ]

formatRow :: Row -> String
formatRow (Prefix row)    = show row
formatRow (Suffix ty row) = unwords [show ty, formatRow row]

formatNames :: Show a => [a] -> String
formatNames names = "(" ++ unwords (map show names) ++ ")\n"

printEquation :: (Supply, RowVar, Row, [Constraint]) -> IO ()
printEquation (s, input, output, equations) = do
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
  mapM_ printEq [(dupBody, "DUP:"), (dropBody, "DROP:"), (rotBody, "ROT:")]
  where printEq (body, name) = do
          putStrLn name
          printEquation $ constrainBody body
