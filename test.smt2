(declare-const x0 String)
(declare-const x1 String)
(declare-const x2 String)
(declare-const x3 String)
(declare-const i Int)
(assert (= x1 (str.++ x0 x0)))
(assert (= x2 (str.++ x1 x0 x1)))
(assert (= x3 (str.replaceallre x2 (str.to.re "abc") "xyz")))
(assert (str.in.re x1 (re.+ (str.to.re "ab"))))
(assert (str.in.re x2 (re.+ (str.to.re "aa"))))
(check-sat)