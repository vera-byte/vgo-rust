-- ================================
-- 微信用户表迁移脚本
-- ================================

-- 微信用户信息表（关联user表）
DROP TABLE IF EXISTS "public"."user_wx";
CREATE TABLE "public"."user_wx" (
  "id" BIGSERIAL PRIMARY KEY,
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "deleted_at" TIMESTAMPTZ NULL,
  "tenant_id" INTEGER,
  
  -- 关联用户表
  "user_id" BIGINT NOT NULL,
  
  -- 微信开放平台信息
  "unionid" VARCHAR(128),
  "openid" VARCHAR(128) NOT NULL,
  
  -- 微信用户信息
  "avatar_url" TEXT,
  "nick_name" VARCHAR(100),
  "gender" SMALLINT NOT NULL DEFAULT 0 CHECK (gender IN (0, 1, 2)),
  "language" VARCHAR(50),
  "city" VARCHAR(100),
  "province" VARCHAR(100),
  "country" VARCHAR(100),
  
  -- 应用类型
  "type" SMALLINT NOT NULL DEFAULT 0 CHECK (type IN (0, 1, 2, 3)),
  
  -- 外键约束
  CONSTRAINT "fk_user_wx_user" FOREIGN KEY ("user_id") 
    REFERENCES "public"."user" ("id") ON DELETE CASCADE
);

-- 索引
CREATE INDEX "idx_user_wx_updated_at" ON "public"."user_wx" ("updated_at");
CREATE INDEX "idx_user_wx_openid" ON "public"."user_wx" ("openid");
CREATE INDEX "idx_user_wx_unionid" ON "public"."user_wx" ("unionid");
CREATE INDEX "idx_user_wx_created_at" ON "public"."user_wx" ("created_at");
CREATE INDEX "idx_user_wx_tenant_id" ON "public"."user_wx" ("tenant_id");
CREATE INDEX "idx_user_wx_user_id" ON "public"."user_wx" ("user_id");

-- 唯一索引
CREATE UNIQUE INDEX "uk_user_wx_openid" ON "public"."user_wx" ("openid") WHERE "deleted_at" IS NULL;
CREATE UNIQUE INDEX "uk_user_wx_unionid_tenant" ON "public"."user_wx" ("unionid", "tenant_id") WHERE "unionid" IS NOT NULL AND "deleted_at" IS NULL;
CREATE UNIQUE INDEX "uk_user_wx_user_type" ON "public"."user_wx" ("user_id", "type") WHERE "user_id" IS NOT NULL AND "deleted_at" IS NULL;

-- 自动更新时间触发器
CREATE TRIGGER trigger_user_wx_update_time
    BEFORE UPDATE ON "public"."user_wx"
    FOR EACH ROW
    EXECUTE FUNCTION public.update_update_time();

-- 表注释
COMMENT ON TABLE "public"."user_wx" IS '微信用户信息表（关联user表）';
COMMENT ON COLUMN "public"."user_wx"."id" IS '主键ID';
COMMENT ON COLUMN "public"."user_wx"."created_at" IS '创建时间';
COMMENT ON COLUMN "public"."user_wx"."updated_at" IS '更新时间';
COMMENT ON COLUMN "public"."user_wx"."deleted_at" IS '软删除时间';
COMMENT ON COLUMN "public"."user_wx"."tenant_id" IS '租户ID';
COMMENT ON COLUMN "public"."user_wx"."user_id" IS '关联用户ID';
COMMENT ON COLUMN "public"."user_wx"."unionid" IS '微信unionid';
COMMENT ON COLUMN "public"."user_wx"."openid" IS '微信openid';
COMMENT ON COLUMN "public"."user_wx"."avatar_url" IS '微信头像URL';
COMMENT ON COLUMN "public"."user_wx"."nick_name" IS '微信昵称';
COMMENT ON COLUMN "public"."user_wx"."gender" IS '微信性别: 0-未知, 1-男, 2-女';
COMMENT ON COLUMN "public"."user_wx"."type" IS '微信类型: 0-小程序, 1-公众号, 2-H5, 3-APP';
