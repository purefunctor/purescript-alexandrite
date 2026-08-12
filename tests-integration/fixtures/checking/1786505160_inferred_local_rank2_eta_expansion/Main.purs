module Main where

data Result = Result

data Spec = Spec

data Query a = Query a

data Eval a = Eval a

mkEval :: forall a. Spec -> Query a -> Eval a
mkEval _ (Query value) = Eval value

accept :: (forall a. Query a -> Eval a) -> Result
accept _ = Result

etaExpansion :: Result
etaExpansion = accept eval
  where
  eval query = mkEval Spec query
