module Main where

data Result = Result

data Spec = Spec

data Query a = Query a

data Eval a = Eval a

data Pair a b = Pair a b

mkEval :: forall a. Spec -> Query a -> Eval a
mkEval _ (Query value) = Eval value

accept :: (forall a. Query a -> Eval a) -> Result
accept _ = Result

concreteReuse = Pair (accept eval) (eval (Query Result))
  where
  eval = mkEval Spec
