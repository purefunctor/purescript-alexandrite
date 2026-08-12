module Main where

data Result = Result

data Spec = Spec

data Query a = Query a

data Eval a = Eval a

type ComponentSpec =
  { eval :: forall a. Query a -> Eval a }

mkEval :: forall a. Spec -> Query a -> Eval a
mkEval _ (Query value) = Eval value

mkComponent :: ComponentSpec -> Result
mkComponent _ = Result

accept :: (forall a. Query a -> Eval a) -> Result
accept _ = Result

direct :: Result
direct = mkComponent { eval }
  where
  eval = mkEval Spec

aliased :: Result
aliased = accept alias
  where
  eval = mkEval Spec
  alias = eval
