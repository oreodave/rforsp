import           Data.Function ((&))
import qualified Data.Set      as Set
import           Types
import           Verifier

normaliseType :: TypeVar -> Analysis -> TypeVar
normaliseType t a =
  Set.findMin $ build_set Set.empty [t]
  where
    build_set seen [] = seen
    build_set seen (first : rest) =
      if Set.member first seen then build_set seen rest
      else let neighbours = [if t1 == first then t2 else t1
                            | TypeEq t1 t2 <- (constraints a),
                              t1 == first || t2 == first] in
             build_set (Set.insert first seen) (neighbours ++ rest)

normaliseRow :: Row -> Analysis -> Row
normaliseRow (Suffix ty row) a =
  Suffix (normaliseType ty a) $ normaliseRow row a
normaliseRow (Prefix row_var) a =
  case constrs of
    [] -> Prefix row_var
    (t, r) : _ | r == row_var ->
      error $ "Recursive constraint detected: " ++ (show $ RowEq row_var t r)
    (t, r) : _ ->
      Suffix (normaliseType t a) $ normaliseRow (Prefix r) a
  where constrs = [(t, r2) | RowEq r1 t r2 <- (constraints a), r1 == row_var]


formatAnalysis :: Analysis -> String
formatAnalysis analysis@(Analysis _ input output equations) =
  unlines $ (:) formatted_sig $ concatMap format_section constraint_sections
  where formatted_sig         = unwords
          [normaliseRow input analysis & show, "--",
           normaliseRow output analysis & show]
        constraint_sections =
          let type_constraints = [c | c@(TypeEq _ _)  <- equations]
              row_constraints  = [c | c@(RowEq _ _ _) <- equations]
          in [("  Type Constraints:", type_constraints),
              ("  Row Constraints:", row_constraints)]
        format_section (heading, constraints) = case constraints of
          [] -> []
          _  -> constraints & map ((++) "    " . show) & (:) heading


bodies :: [(String, Body)]
bodies = [
  ("Self assign",
   Body 1 [
      Block [Bind 0, Load Local 0, Bind 0]
        End
   ]),
  ("2 way assign",
   Body 2 [
      Block [Bind 0, Load Local 0, Bind 1, Load Local 1, Bind 0]
        End
   ]),
  ("3 sequential assign",
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
  where printEq (name, body) =
          putStrLn $ name ++ ": " ++ (formatAnalysis $ constrainBody body)
