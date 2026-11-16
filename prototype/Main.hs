{-# LANGUAGE TemplateHaskell #-}
import           Data.List (find, foldl)
import           Types
import           Verifier

formatRow :: Row -> String
formatRow (Prefix row)    = show row
formatRow (Suffix ty row) = unwords [show ty, formatRow row]

formatNames :: Show a => [a] -> String
formatNames names = "(" ++ unwords (map show names) ++ ")\n"

formatConstraint :: Constraint -> String
formatConstraint x = "  " ++ show x

printEquation :: Analysis -> IO ()
printEquation (Analysis s input output equations) = do
  let type_constraints   = [TypeEq a b  | TypeEq a b  <- equations]
      row_constraints    = [RowEq a t b | RowEq a t b <- equations]
      format_constraints = unlines . map formatConstraint
    in do
      putStrLn $ (show input) ++ " -- " ++ (show output);
      putStrLn $ "Type Constraints:\n" ++ format_constraints type_constraints;
      putStrLn $ "Row Constraints:\n" ++ format_constraints row_constraints;

bodies :: [(String, Body)]
bodies = [
  ("DROP",
   Body 1 [
      Block [Bind 0] End
   ]),
  ("DUP",
   Body 1 [
      Block [Bind 0, Load Local 0, Load Local 0] End
   ]),
  ("ROT",
   Body 3 [
      Block [Bind 0, Bind 1, Bind 2,
             Load Local 1, Load Local 0, Load Local 2]
        End
   ])
  ]

main :: IO ()
main =
  mapM_ printEq bodies
  where printEq (name, body) = do
          putStrLn name
          printEquation $ constrainBody body
