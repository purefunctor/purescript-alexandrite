module Main where

data Result = Result

data Spec = Spec

data Query a = Query a

data Eval a = Eval a

identity :: forall a. a -> a
identity value = value

mkEval :: forall a. Spec -> Query a -> Eval a
mkEval _ (Query value) = Eval value

accept :: (forall a. Query a -> Eval a) -> Result
accept _ = Result

nestedApplication :: Result
nestedApplication = accept eval
  where
  eval = identity (mkEval Spec)
