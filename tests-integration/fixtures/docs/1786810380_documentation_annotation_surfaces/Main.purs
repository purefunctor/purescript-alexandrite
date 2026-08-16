-- | Documentation across
-- | multiple module lines.
module Main where

foreign import data Proxy :: forall k. k -> Type

-- | Foreign value documentation.
foreign import foreignValue :: Int

-- | Pair type documentation.
data Pair a b = Pair a b

-- | Type operator documentation.
infixr 6 type Pair as :+:

-- | Choice type documentation.
data Choice
  -- | First constructor documentation.
  = First
  -- | Second constructor documentation.
  | Second

append :: String -> String -> String
append left _ = left

-- | Value operator documentation.
infixr 5 append as <>

equationDocumented :: Int
-- | Equation documentation.
equationDocumented = 1

-- | Signature documentation takes precedence.
signatureDocumented :: Int
-- | Ignored equation documentation.
signatureDocumented = 2

-- | Type literals and open rows.
foreign import typeShapes
  :: forall row
   . Proxy (field :: Int | row)
  -> Proxy 42
  -> Proxy "label"
  -> Proxy (Int :: Type)
  -> Int
