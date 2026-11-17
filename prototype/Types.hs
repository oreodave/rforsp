module Types where

newtype TypeVar = TypeVar Int deriving (Eq, Ord)
instance Show TypeVar where
  show (TypeVar n) = "T" ++ show n


newtype RowVar = RowVar Int deriving (Eq, Ord)
instance Show RowVar where
  show (RowVar n) = "R" ++ show n


data Constraint =
  TypeEq TypeVar TypeVar
  | RowEq RowVar TypeVar RowVar

instance Eq Constraint where
  (TypeEq a b) == (TypeEq c d) = (a == c && b == d) || (a == d && b == c)
  (RowEq r11 t1 r12) == (RowEq r21 t2 r22) =
    r11 == r21 && t1 == t2 && r12 == r22
  _ == _ = False

instance Show Constraint where
  show (TypeEq t1 t2)  = unwords [show t1, "=", show t2]
  show (RowEq r1 t r2) = unwords [show r1, "=", show t, show r2]


data Row = Prefix RowVar | Suffix TypeVar Row
instance Show Row where
  show (Prefix x)   = show x
  show (Suffix t x) = show t ++ " " ++ show x


data LoadKind = Local | Captured
data Instruction = Bind Int | Load LoadKind Int
data Terminator = End | Branch Int Int | Jump Int
data Block = Block [Instruction] Terminator
data Body = Body Int [Block]


data Supply = Supply
  { nextType  :: Int
  , nextRow   :: Int
  , typeTable :: [TypeVar]
  , rowTable  :: [RowVar]
  }


emptySupply :: Supply
emptySupply = Supply 0 0 [] []


freshType :: Supply -> (Supply, TypeVar)
freshType s =
  let v = TypeVar (nextType s) in
    (s {nextType = nextType s + 1, typeTable = v : typeTable s}, v)


freshRow :: Supply -> (Supply, RowVar)
freshRow s =
  let v = RowVar (nextRow s) in
    (s {nextRow = nextRow s + 1, rowTable = v : rowTable s}, v)
