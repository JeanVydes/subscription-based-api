# API integrate with LemonSqueezy Template
´´´{{{{{{{{{{{{{{}}}}}}}}}}}}}}
Check the [Postman API Reference](https://www.postman.com/jeanservices/workspace/8i/collection/11966073-e41fc689-a391-45b6-8d1e-d2b6176b5615?action=share&creator=11966073) to learn how to use

# Run

* Fly.io as Hosting Provider (optional)
* MongoDB Instance (mongodb.com)
* RedisDB Instance (upstash.io)
* Set enviroment vars
* `fly launch`

# .env Template

```
HOST=0.0.0.0
PORT=8080

MONGO_URI=                          # fly secrets set MONGO_URI=
REDIS_URI=                          # fly secrets set REDIS_URI=
MONGO_DB_NAME=

API_TOKENS_SIGNING_KEY=             # fly secrets set API_TOKENS_SIGNING_KEY=
API_TOKENS_EXPIRATION_TIME=

LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY= # fly secrets set LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY=
PRO_PRODUCT_ID=
PRO_MONTHLY_VARIANT_ID=
PRO_ANNUALLY_VARIANT_ID=
```

# Help

Reach me via email `github@jeanvides.com`