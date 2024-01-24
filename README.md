# Start Building Your API With This Complete Template Ready For Production

start creating, don't worry about boring stuff

this is a base for start a powerful API using axum, databases and subscriptions, including useful stuff like accounts managing with sessions, just start creating your idea, dont worry about low level boring stuff

i tried to make this the most organized possible, organizing per modules, so you don't have to break your mind trying to comprend the api functionability

also i create a postman api reference, you have to replicate that in your frontend and you are ready, check the [Postman API Reference](https://www.postman.com/jeanservices/workspace/8i/collection/11966073-e41fc689-a391-45b6-8d1e-d2b6176b5615?action=share&creator=11966073) to learn how to use

# Features

* Accounts System
* OAuth using Google
* Login System Based In Session Tokens
* Database Connection with Postgres (misc) using Diesel as ORM, MongoDB (for users) and Redis (for sessions)
* LemonSqueezy Subscription Totally Integration
* The API provide ratelimits per routers, buffering, CORS, compression, fallbacks, and that's boring stuff

# Run

* Fly.io as Hosting Provider (optional)
* PostgresSQL Instance (optional [SupaBase](https://supabase.com/))
* MongoDB Instance ([MongoDB](https://mongodb.com/))
* RedisDB Instance ([upstash.io](https://upstash.io/))
* Set enviroment vars
* `fly launch`

# .env Template

```
HOST=0.0.0.0                            # Not Sensitive Data (fly.toml)
PORT=8080                               # Not Sensitive Data (fly.toml)

API_URL=                                # Not Sensitive Data (fly.toml)

POSTGRES_URI=                           # (optional) fly secrets set POSTGRES_URI=
MONGO_URI=                              # fly secrets set MONGO_URI=
REDIS_URI=                              # fly secrets set REDIS_URI=
MONGO_DB_NAME=                          #  Not Sensitive Data (fly.toml)

API_TOKENS_SIGNING_KEY=                 # fly secrets set API_TOKENS_SIGNING_KEY=
API_TOKENS_EXPIRATION_TIME=

LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY=     # fly secrets set LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY=
PRO_PRODUCT_ID=                         # Not Sensitive Data (fly.toml)
PRO_MONTHLY_VARIANT_ID=                 # Not Sensitive Data (fly.toml)
PRO_ANNUALLY_VARIANT_ID=                # Not Sensitive Data (fly.toml)

ENABLE_EMAIL_VERIFICATION=              # Not Sensitive Data (fly.toml)

BREVO_CUSTOMERS_WEBFLOW_API_KEY=        # fly secrets set 
BREVO_CUSTOMERS_LIST_ID=                # Not Sensitive Data (fly.toml)
BREVO_EMAIL_VERIFY_TEMPLATE_ID=         # Not Sensitive Data (fly.toml)

BREVO_MASTER_EMAIL_ADDRESS=             # Not Sensitive Data (fly.toml)
BREVO_MASTER_NAME=                      # Not Sensitive Data (fly.toml)

GOOGLE_OAUTH_CLIENT_ID=                 # Not Sensitive Data (fly.toml)
GOOGLE_OAUTH_CLIENT_SECRET=             # fly secrets set GOOGLE_OAUTH_CLIENT_SECRET= 
GOOGLE_OAUTH_CLIENT_REDIRECT_ENDPOINT=  # Not Sensitive Data (fly.toml)
```

# Future Ideas

* Next.js template
* OAuth with Apple

# Help

Reach me via email `github@jeanvides.com`