module Alexandrite.StyleX
  ( Style
  , Props
  , Keyframes
  , create
  , props
  , keyframes
  ) where

import Prim.Row as Row
import Prim.RowList as RowList

data Style :: Type
data Style

type Props = { className :: String }

data Keyframes :: Type
data Keyframes

class CompileStyles :: Row Type -> Row Type -> Constraint
class CompileStyles input output | input -> output

class CompileStyleList :: RowList.RowList Type -> Row Type -> Constraint
class CompileStyleList input output | input -> output

instance
  ( RowList.RowToList input inputList
  , CompileStyleList inputList output
  ) =>
  CompileStyles input output

instance CompileStyleList RowList.Nil ()

instance
  ( CompileStyleList tail outputTail
  , Row.Cons label Style outputTail output
  ) =>
  CompileStyleList (RowList.Cons label (Record declarations) tail) output

foreign import create
  :: forall input output
   . CompileStyles input output
  => Record input
  -> Record output

foreign import props :: Style -> Props

foreign import keyframes :: forall frames. Record frames -> Keyframes
