module BindOperator ((>>=)) where

import Control.Bind (bind)

infixl 1 bind as >>=
