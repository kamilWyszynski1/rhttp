enum Outcome<S, E, F> {
    Success(S),
    Failure(E),
    Forward(F),
}
