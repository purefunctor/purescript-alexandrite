module Main where

class Named a where
  name :: String

instance Named String where
  name = "x"

select :: forall @fixed @selected. Named selected => String
select = name @selected

test :: String
test = alias @String
  where
  alias = select @String
