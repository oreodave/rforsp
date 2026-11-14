module Types where

newtype TypeVar = TypeVar Int deriving (Eq)
newtype RowVar = RowVar Int deriving (Eq)

instance Show TypeVar where
  show (TypeVar n) = "T" ++ show n

instance Show RowVar where
  show (RowVar n) = "R" ++ show n

data Constraint =
  TypeEq TypeVar TypeVar
  | RowEq RowVar TypeVar RowVar
  deriving (Show)

data Row = Prefix RowVar | Suffix TypeVar Row

instance Show Row where
  show (Prefix x)   = show x
  show (Suffix t x) = show t ++ " " ++ show x

data LoadKind = Local | Captured
data Instruction = Bind Int | Load LoadKind Int
data Terminator = End | Branch Int Int | Jump Int
data Block = Block [Instruction] Terminator
data Body = Body Int [Block]

-- Body 1 of sir/std.sexp is dup.
dupBody :: Body
dupBody =
  Body 1 [
  Block [Bind 0, Load Local 0, Load Local 0] End
  ]

data Supply = Supply
  { nextType  :: Int
  , nextRow   :: Int
  , typeTable :: [TypeVar]
  , rowTable  :: [RowVar]
  }

emptySupply :: Supply
emptySupply = Supply 0 0 [] []

freshType :: Supply -> (TypeVar, Supply)
freshType s =
  let v = TypeVar (nextType s)
   in (v, s {nextType = nextType s + 1, typeTable = v : typeTable s})

freshRow :: Supply -> (RowVar, Supply)
freshRow s =
  let v = RowVar (nextRow s)
   in (v, s {nextRow = nextRow s + 1, rowTable = v : rowTable s})
