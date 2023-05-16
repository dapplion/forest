use std::path::Path;

use forest_paramfetch::{get_params, SectorSizeOpt};

#[tokio::test]
async fn download_params() {
    let params_json = "{
    \"v28-fil-inner-product-v1.srs\": {
        \"cid\": \"Qmdq44DjcQnFfU3PJcdX7J49GCqcUYszr1TxMbHtAkvQ3g\",
        \"digest\": \"ae20310138f5ba81451d723f858e3797\",
        \"sector_size\": 0
    }
}";

    get_params(
        Path::new("parameters_test"),
        params_json,
        SectorSizeOpt::All,
    )
    .await
    .unwrap();
}
