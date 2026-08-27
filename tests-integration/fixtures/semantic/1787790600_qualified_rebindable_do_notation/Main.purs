module Main where

import Control as Control

qualifiedDo :: Control.Box Int
qualifiedDo = Control.do
  value <- Control.action
  Control.action
  Control.pure value

qualifiedAdo :: Control.Box Int
qualifiedAdo = Control.ado
  left <- Control.action
  right <- Control.action
  in left

qualifiedPure :: Control.Box Int
qualifiedPure = Control.ado
  in 1
