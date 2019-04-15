/// Anonymous struct with named fields.
macro_rules! conf {
    ( $( $id:ident : $init:expr, )* ) => (
        {
            #[allow(non_camel_case_types)]
            struct Conf<$( $id ),*> {
                $( $id : $id ),*
            }
            Conf {
                $( $id: $init ),*
            }
        }
    )
}
