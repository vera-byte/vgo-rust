-- OAuth2 应用表 / OAuth2 Application table
-- 说明: 管理 OAuth2 Client 的基础信息、授权配置与审计字段
-- Description: Manage OAuth2 clients, authorization settings and audit fields

CREATE TABLE IF NOT EXISTS public.oauth2_app (
  id                BIGSERIAL PRIMARY KEY,
  tenant_id         BIGINT NULL,
  name              VARCHAR(80) NOT NULL,
  client_id         VARCHAR(64) NOT NULL,
  client_secret     VARCHAR(128) NULL,
  confidentiality   SMALLINT NOT NULL DEFAULT 1,
  first_party       BOOLEAN NOT NULL DEFAULT FALSE,
  status            SMALLINT NOT NULL DEFAULT 1,
  revoked_at        TIMESTAMPTZ NULL,
  redirect_uris     TEXT[] NOT NULL DEFAULT '{}',
  grant_types       TEXT[] NOT NULL DEFAULT '{authorization_code}',
  response_types    TEXT[] NOT NULL DEFAULT '{code}',
  scopes            TEXT[] NOT NULL DEFAULT '{}',
  token_auth_method TEXT NOT NULL DEFAULT 'client_secret_basic',
  logo_url          TEXT NULL,
  website_url       TEXT NULL,
  contacts          JSONB NOT NULL DEFAULT '[]',
  metadata          JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  deleted_at        TIMESTAMPTZ NULL,

  CONSTRAINT ck_confidential_secret
    CHECK (
      confidentiality IN (0,1)
      AND (
        confidentiality = 0
        OR token_auth_method = 'private_key_jwt'
        OR client_secret IS NOT NULL
      )
    ),
  CONSTRAINT ck_status CHECK (status IN (0,1,2)),
  CONSTRAINT ck_grant_types
    CHECK (grant_types <@ ARRAY['authorization_code','client_credentials','refresh_token','password','device_code']),
  CONSTRAINT ck_response_types
    CHECK (response_types <@ ARRAY['code','token','id_token']),
  CONSTRAINT ck_token_auth_method
    CHECK (token_auth_method IN ('client_secret_basic','client_secret_post','private_key_jwt','none'))
);

-- 表注释 / Table comment
COMMENT ON TABLE public.oauth2_app IS 'OAuth2 应用表：管理客户端和授权配置 / OAuth2 application table: manage clients and authorization settings';

-- 字段注释 / Column comments
COMMENT ON COLUMN public.oauth2_app.id                IS '主键，自增 / Primary key, auto-increment';
COMMENT ON COLUMN public.oauth2_app.tenant_id         IS '租户ID（多租户）/ Tenant ID (multi-tenancy)';
COMMENT ON COLUMN public.oauth2_app.name              IS '应用名称 / Application display name';
COMMENT ON COLUMN public.oauth2_app.client_id         IS '客户端标识 / OAuth2 client identifier';
COMMENT ON COLUMN public.oauth2_app.client_secret     IS '客户端密钥（机密客户端必需）/ Client secret (required for confidential clients)';
COMMENT ON COLUMN public.oauth2_app.confidentiality   IS '机密性：0=公开，1=机密 / Confidentiality: 0=public, 1=confidential';
COMMENT ON COLUMN public.oauth2_app.first_party       IS '第一方应用标识 / First-party application flag';
COMMENT ON COLUMN public.oauth2_app.status            IS '状态：0=禁用，1=启用，2=冻结 / Status: 0=disabled, 1=enabled, 2=suspended';
COMMENT ON COLUMN public.oauth2_app.revoked_at        IS '撤销时间 / Revoked at timestamp';
COMMENT ON COLUMN public.oauth2_app.redirect_uris     IS '授权回调地址列表 / Allowed redirect URIs';
COMMENT ON COLUMN public.oauth2_app.grant_types       IS '允许的授权类型 / Allowed grant types';
COMMENT ON COLUMN public.oauth2_app.response_types    IS '允许的响应类型 / Allowed response types';
COMMENT ON COLUMN public.oauth2_app.scopes            IS '允许的范围列表 / Allowed scopes';
COMMENT ON COLUMN public.oauth2_app.token_auth_method IS '令牌端点认证方式 / Token endpoint client authentication method';
COMMENT ON COLUMN public.oauth2_app.logo_url          IS '应用 Logo 地址 / App logo URL';
COMMENT ON COLUMN public.oauth2_app.website_url       IS '应用网站地址 / App website URL';
COMMENT ON COLUMN public.oauth2_app.contacts          IS '联系人信息（数组） / Contacts information (array)';
COMMENT ON COLUMN public.oauth2_app.metadata          IS '自定义元数据（JSON）/ Custom metadata (JSON)';
COMMENT ON COLUMN public.oauth2_app.created_at        IS '创建时间 / Created at timestamp';
COMMENT ON COLUMN public.oauth2_app.updated_at        IS '更新时间（触发器自动维护） / Updated at timestamp (auto by trigger)';
COMMENT ON COLUMN public.oauth2_app.deleted_at        IS '软删除时间 / Soft deleted timestamp';

-- 唯一约束 / Unique constraints
CREATE UNIQUE INDEX IF NOT EXISTS uk_oauth2_app_client
  ON public.oauth2_app (tenant_id, client_id)
  WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uk_oauth2_app_name
  ON public.oauth2_app (tenant_id, name)
  WHERE deleted_at IS NULL;

-- 性能索引 / Performance indexes
CREATE INDEX IF NOT EXISTS idx_oauth2_app_status ON public.oauth2_app (status);
CREATE INDEX IF NOT EXISTS idx_oauth2_app_updated_at ON public.oauth2_app (updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_oauth2_app_redirect_uris ON public.oauth2_app USING GIN (redirect_uris);
CREATE INDEX IF NOT EXISTS idx_oauth2_app_grant_types   ON public.oauth2_app USING GIN (grant_types);
CREATE INDEX IF NOT EXISTS idx_oauth2_app_scopes        ON public.oauth2_app USING GIN (scopes);

-- 更新时间触发器（存在则创建）/ updated_at trigger (create if function exists)
DO $$ BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_proc WHERE proname = 'update_update_time' AND pg_function_is_visible(oid)
  ) THEN
    CREATE TRIGGER trigger_oauth2_app_update_time
      BEFORE UPDATE ON public.oauth2_app
      FOR EACH ROW EXECUTE FUNCTION public.update_update_time();
  END IF;
END $$;


-- 测试数据 / Test data
-- INSERT INTO public.oauth2_app (
--   tenant_id, name, client_id, client_secret, confidentiality, first_party, status,
--   revoked_at, redirect_uris, grant_types, response_types, scopes,
--   token_auth_method, logo_url, website_url, contacts, metadata
-- ) VALUES
-- (1, 'First-Party Web App', 'fp_web_app', 'fp-web-secret-123', 1, TRUE, 1,
--  NULL, ARRAY['https://web.example.com/callback'], ARRAY['authorization_code','refresh_token'], ARRAY['code'],
--  ARRAY['openid','profile','email','offline_access'], 'client_secret_basic',
--  'https://cdn.example.com/logo/web.png', 'https://web.example.com',
--  '[{"name":"Alice","email":"alice@example.com"}]'::jsonb,
--  '{"category":"web","notes":"uses PKCE optionally"}'::jsonb),

-- (1, 'Public SPA', 'public_spa', NULL, 0, FALSE, 1,
--  NULL, ARRAY['https://spa.example.com/callback'], ARRAY['authorization_code'], ARRAY['code'],
--  ARRAY['openid','profile','email'], 'none',
--  'https://cdn.example.com/logo/spa.png', 'https://spa.example.com',
--  '[{"name":"Bob","email":"bob@example.com"}]'::jsonb,
--  '{"category":"spa","pkce":true}'::jsonb),

-- (1, 'M2M Service', 'm2m_service', 'm2m-secret-456', 1, FALSE, 1,
--  NULL, ARRAY[]::text[], ARRAY['client_credentials'], ARRAY['code'],
--  ARRAY['read','write'], 'client_secret_basic',
--  'https://cdn.example.com/logo/m2m.png', 'https://service.example.com',
--  '[{"name":"ServiceOwner","email":"owner@example.com"}]'::jsonb,
--  '{"category":"service","audience":"api"}'::jsonb),

-- (1, 'Device App', 'device_app', 'device-secret-789', 1, FALSE, 1,
--  NULL, ARRAY[]::text[], ARRAY['device_code'], ARRAY['code'],
--  ARRAY['openid','offline_access'], 'client_secret_post',
--  'https://cdn.example.com/logo/device.png', 'https://device.example.com',
--  '[{"name":"Support","email":"support@example.com"}]'::jsonb,
--  '{"category":"device","platform":"tv"}'::jsonb),

-- (1, 'Revoked App', 'revoked_app', 'revoked-secret-000', 1, FALSE, 0,
--  NOW() - INTERVAL '7 days', ARRAY['https://rev.example.com/callback'], ARRAY['authorization_code'], ARRAY['code'],
--  ARRAY['openid'], 'client_secret_basic',
--  'https://cdn.example.com/logo/rev.png', 'https://rev.example.com',
--  '[{"name":"Ops","email":"ops@example.com"}]'::jsonb,
--  '{"reason":"security incident"}'::jsonb),

-- (1, 'Suspended App', 'suspended_app', 'suspend-secret-111', 1, FALSE, 2,
--  NULL, ARRAY['https://sus.example.com/callback'], ARRAY['authorization_code','refresh_token'], ARRAY['code'],
--  ARRAY['openid','profile'], 'client_secret_post',
--  'https://cdn.example.com/logo/sus.png', 'https://sus.example.com',
--  '[{"name":"Compliance","email":"comp@example.com"}]'::jsonb,
--  '{"suspend_reason":"policy violation"}'::jsonb),

-- (1, 'Multi Redirect App', 'multi_redirect_app', 'multi-secret-222', 1, FALSE, 1,
--  NULL, ARRAY['https://app.example.com/callback','https://app.example.com/cb2'], ARRAY['authorization_code'], ARRAY['code'],
--  ARRAY['openid','email'], 'client_secret_basic',
--  'https://cdn.example.com/logo/multi.png', 'https://app.example.com',
--  '[{"name":"Dev","email":"dev@example.com"}]'::jsonb,
--  '{"notes":"multiple redirect URIs"}'::jsonb),

-- (1, 'Private Key JWT App', 'pk_jwt_app', NULL, 1, FALSE, 1,
--  NULL, ARRAY['https://jwt.example.com/callback'], ARRAY['authorization_code','refresh_token'], ARRAY['code'],
--  ARRAY['openid','profile','offline_access'], 'private_key_jwt',
--  'https://cdn.example.com/logo/jwt.png', 'https://jwt.example.com',
--  '[{"name":"Sec","email":"sec@example.com"}]'::jsonb,
--  '{"jwks_uri":"https://jwt.example.com/.well-known/jwks.json"}'::jsonb),

-- (1, 'Secret Post Web', 'secret_post_web', 'post-secret-333', 1, TRUE, 1,
--  NULL, ARRAY['https://post.example.com/callback'], ARRAY['authorization_code','refresh_token'], ARRAY['code'],
--  ARRAY['openid','email','offline_access'], 'client_secret_post',
--  'https://cdn.example.com/logo/post.png', 'https://post.example.com',
--  '[{"name":"Admin","email":"admin@example.com"}]'::jsonb,
--  '{"notes":"uses client_secret_post"}'::jsonb),

-- (2, 'Tenant2 App', 'tenant2_app', 't2-secret-444', 1, FALSE, 1,
--  NULL, ARRAY['https://t2.example.com/callback'], ARRAY['password','refresh_token'], ARRAY['code'],
--  ARRAY['read','write','openid'], 'client_secret_basic',
--  'https://cdn.example.com/logo/t2.png', 'https://t2.example.com',
--  '[{"name":"T2Owner","email":"t2@example.com"}]'::jsonb,
--  '{"category":"mobile","platform":"android"}'::jsonb);