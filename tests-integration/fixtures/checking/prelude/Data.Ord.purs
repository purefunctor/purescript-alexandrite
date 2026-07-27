module Data.Ord where

import Data.Eq (class Eq, class Eq1, class EqRecord)
import Data.Ordering (Ordering(..))
import Data.Symbol (class IsSymbol)
import Prim.Row as Row
import Prim.RowList as RL

class Eq a <= Ord a where
  compare :: a -> a -> Ordering

class Eq1 f <= Ord1 f where
  compare1 :: forall a. Ord a => f a -> f a -> Ordering

instance Ord Int where
  compare _ _ = EQ

instance Ord Boolean where
  compare _ _ = EQ

instance ordRec :: (RL.RowToList row list, OrdRecord list row) => Ord (Record row) where
  compare _ _ = EQ

class OrdRecord :: RL.RowList Type -> Row Type -> Constraint
class EqRecord rowlist row <= OrdRecord rowlist row where
  compareRecord :: rowlist -> Record row -> Record row -> Ordering

instance OrdRecord RL.Nil row where
  compareRecord _ _ _ = EQ

instance
  ( OrdRecord rowlistTail row
  , Row.Cons key focus rowTail row
  , IsSymbol key
  , Ord focus
  ) =>
  OrdRecord (RL.Cons key focus rowlistTail) row where
  compareRecord _ _ _ = EQ
