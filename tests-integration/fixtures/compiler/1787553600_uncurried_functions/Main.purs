module Main where

import Data.Function.Uncurried (Fn2, Fn3, mkFn2, mkFn3, runFn2, runFn3)
import Lookalike as Lookalike

foreign import chooseSecond :: Fn2 Int Int Int

made :: Fn2 Int Int Int
made = mkFn2 \first second -> second

madeNested :: Fn3 Int Int Int Int
madeNested = mkFn3 \first -> \second third -> third

madeCaptured :: Int -> Fn2 Int Int Int
madeCaptured captured = mkFn2 \first second -> captured

madeWithCurriedResult :: Fn2 Int Int (Int -> Int)
madeWithCurriedResult = mkFn2 \first second third -> third

directRun :: Int
directRun = runFn2 chooseSecond 1 42

directMadeRun :: Int
directMadeRun = runFn2 (mkFn2 (\first second -> second)) 1 42

directNestedRun :: Int
directNestedRun = runFn3 madeNested 1 2 42

directCapturedRun :: Int
directCapturedRun = runFn2 (madeCaptured 42) 1 2

directCurriedResultRun :: Int
directCurriedResultRun = runFn2 madeWithCurriedResult 1 2 42

partialRun :: Int -> Int
partialRun = runFn2 chooseSecond 1

indirectMake :: Fn2 Int Int Int
indirectMake = mkFn2 chooseSecondCurried

chooseSecondCurried :: Int -> Int -> Int
chooseSecondCurried first second = second

lookalikeMade :: Int -> Int -> Int
lookalikeMade = Lookalike.mkFn2 \first second -> second

lookalikeRun :: Int
lookalikeRun = Lookalike.runFn2 lookalikeMade 1 42
