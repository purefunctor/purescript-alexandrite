module Main where

data Result = Result

data Spec = Spec

data Query a = Query a

data Eval a = Eval a

class Operation a where
  operation :: a -> a

mkEval :: forall a. Operation a => Spec -> Query a -> Eval a
mkEval _ (Query value) = Eval (operation value)

accept :: (forall a. Operation a => Query a -> Eval a) -> Result
accept _ = Result

constrainedLocal :: Result
constrainedLocal = accept eval
  where
  eval = mkEval Spec
