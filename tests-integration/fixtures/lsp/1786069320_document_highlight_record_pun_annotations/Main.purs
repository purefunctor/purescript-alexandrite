module Main where

extract { value } = value
--        &         &

extractComment { {- thing -} value } = value
--                               &         &
