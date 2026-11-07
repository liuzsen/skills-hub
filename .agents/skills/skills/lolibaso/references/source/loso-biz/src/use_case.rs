use super::result::BizResult;

pub trait UseCase: Send {
    type Input;

    type Output;

    type Error;

    fn execute(
        self,
        input: Self::Input,
    ) -> impl Future<Output = BizResult<Self::Output, Self::Error>> + Send;
}
