module Main where

import Type.Proxy (Proxy(..))

class Encode :: Type -> Type -> Constraint
class Encode token a

class RecordDef :: Type -> Row Type -> Constraint
class RecordDef token row where
  recordDef
    :: { handle :: forall a. Encode token a => token -> Proxy a -> String }
    -> token
    -> Record row
    -> String

class RecordDefRowList :: Type -> Row Type -> Type -> Constraint
class RecordDefRowList token row rowList where
  recordDefRowList
    :: { handle :: forall a. Encode token a => token -> Proxy a -> String }
    -> token
    -> Record row
    -> Proxy rowList
    -> String

instance RecordDefRowList token row rowList => RecordDef token row where
  recordDef interface token row =
    recordDefRowList interface token row (Proxy :: _ rowList)
