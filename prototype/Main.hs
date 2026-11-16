{-# LANGUAGE TemplateHaskell #-}
import           Data.List (find, foldl)
import           Types
import           Verifier

normaliseType :: TypeVar -> Analysis -> TypeVar
normaliseType t a =
  case matched of
    Nothing              -> t
    Just (TypeEq _ next) -> normaliseType next a
  where matchingConstraint (TypeEq t1 t2) = t1 == t
        matchingConstraint _              = False
        matched = find matchingConstraint $ constraints a

normaliseRow :: Row -> Analysis -> Row
normaliseRow (Suffix ty row) a =
  Suffix (normaliseType ty a) $ normaliseRow row a
normaliseRow (Prefix row_var) a =
  case matched of
    Nothing               -> Prefix row_var
    Just (RowEq _ t next) -> Suffix (normaliseType t a) $
                             normaliseRow (Prefix next) a
  where matchingConstraint (RowEq r1 _ r2) = r1 == row_var
        matchingConstraint _               = False
        matched = find matchingConstraint $ constraints a

printEquation :: Analysis -> IO ()
printEquation (Analysis s input output equations) = do
  let a                  = Analysis s input output equations
      type_constraints   = [TypeEq a b  | TypeEq a b  <- equations]
      row_constraints    = [RowEq a t b | RowEq a t b <- equations]
      format_row row     = show $ normaliseRow row a
      format_constraints = unlines . map (\x -> "  " ++ show x)
    in do
      putStrLn $ (format_row input) ++ " -- " ++ (format_row output);
      putStrLn $ "Type Constraints:\n" ++ format_constraints type_constraints;
      putStrLn $ "Row Constraints:\n" ++ format_constraints row_constraints;

bodies :: [(String, Body)]
bodies = [
  ("3-way-assign",
   Body 3 [
      Block [Bind 0, Load Local 0,
             Bind 1, Load Local 1,
             Bind 2, Load Local 2]
        End
   ]),
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
          putStrLn $ name ++ ": "
          printEquation $ constrainBody body
