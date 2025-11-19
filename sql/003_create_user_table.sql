-- ================================
-- 个人用户表迁移脚本
-- 创建user表存储用户基本信息
-- ================================

-- ----------------------------
-- 表结构：用户基本信息表
-- 功能：存储系统用户的核心信息和账户数据
-- ----------------------------
DROP TABLE IF EXISTS "public"."user";
CREATE TABLE "public"."user" (
  -- 主键标识
  "id" BIGSERIAL PRIMARY KEY,
  
  -- 时间戳字段（统一命名） / Unified audit timestamps
  "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  "deleted_at" TIMESTAMPTZ NULL,
  
  -- 租户信息
  "tenant_id" INTEGER,
  
  -- 账户信息
  "username" VARCHAR(50) NOT NULL,
  "email" VARCHAR(100),
  "mobile" VARCHAR(20),
  "password_hash" VARCHAR(255) NOT NULL,
  
  -- 个人信息
  "real_name" VARCHAR(50),
  "nick_name" VARCHAR(50),
  "avatar_url" TEXT,
  "gender" SMALLINT NOT NULL DEFAULT 0 CHECK (gender IN (0, 1, 2)),
  "birthday" DATE,
  "id_card" VARCHAR(18),
  
  -- 联系信息
  "country" VARCHAR(50) DEFAULT '中国',
  "province" VARCHAR(50),
  "city" VARCHAR(50),
  "district" VARCHAR(50),
  "address" TEXT,
  "postal_code" VARCHAR(10),
  
  -- 账户状态
  "status" SMALLINT NOT NULL DEFAULT 1 CHECK (status IN (0, 1, 2)),
  "is_email_verified" BOOLEAN NOT NULL DEFAULT FALSE,
  "is_mobile_verified" BOOLEAN NOT NULL DEFAULT FALSE,
  
  -- 系统信息
  "last_login_time" TIMESTAMPTZ,
  "last_login_ip" INET,
  "login_count" INTEGER NOT NULL DEFAULT 0,
  
  -- 扩展字段
  "source" VARCHAR(20) DEFAULT 'web',
  "invite_code" VARCHAR(20),
  "referrer_id" BIGINT,
  
  -- 备注信息
  "remark" TEXT
);

-- ----------------------------
-- 索引结构
-- 功能：优化查询性能，确保数据唯一性
-- ----------------------------

-- 基础索引
CREATE INDEX "idx_user_tenant_id" ON "public"."user" ("tenant_id");
CREATE INDEX "idx_user_created_at" ON "public"."user" ("created_at");
CREATE INDEX "idx_user_updated_at" ON "public"."user" ("updated_at");
CREATE INDEX "idx_user_status" ON "public"."user" ("status");

-- 账户唯一性索引
CREATE UNIQUE INDEX "uk_user_username_tenant" ON "public"."user" ("username", "tenant_id") WHERE "deleted_at" IS NULL;
CREATE UNIQUE INDEX "uk_user_email" ON "public"."user" ("email") WHERE "email" IS NOT NULL AND "deleted_at" IS NULL;
CREATE UNIQUE INDEX "uk_user_mobile" ON "public"."user" ("mobile") WHERE "mobile" IS NOT NULL AND "deleted_at" IS NULL;
CREATE UNIQUE INDEX "uk_user_id_card" ON "public"."user" ("id_card") WHERE "id_card" IS NOT NULL AND "deleted_at" IS NULL;

-- 查询优化索引
CREATE INDEX "idx_user_real_name" ON "public"."user" ("real_name");
CREATE INDEX "idx_user_nick_name" ON "public"."user" ("nick_name");
CREATE INDEX "idx_user_gender" ON "public"."user" ("gender");
CREATE INDEX "idx_user_birthday" ON "public"."user" ("birthday");
CREATE INDEX "idx_user_referrer" ON "public"."user" ("referrer_id");
CREATE INDEX "idx_user_invite_code" ON "public"."user" ("invite_code");

-- ----------------------------
-- 应用公共触发器
-- 功能：自动维护update_time字段的更新时间
-- ----------------------------
CREATE TRIGGER trigger_user_update_time
    BEFORE UPDATE ON "public"."user"
    FOR EACH ROW
    EXECUTE FUNCTION public.update_update_time();

-- ----------------------------
-- 外键约束（可选）
-- 功能：确保数据引用完整性
-- ----------------------------
-- ALTER TABLE "public"."user" 
-- ADD CONSTRAINT "fk_user_referrer" 
-- FOREIGN KEY ("referrer_id") REFERENCES "public"."user" ("id") 
-- ON DELETE SET NULL;

-- ----------------------------
-- 表和字段注释
-- 功能：提供数据字典和文档说明
-- ----------------------------
COMMENT ON TABLE "public"."user" IS '用户基本信息表';

-- 主键和时间
COMMENT ON COLUMN "public"."user"."id" IS '主键ID';
COMMENT ON COLUMN "public"."user"."created_at" IS '创建时间';
COMMENT ON COLUMN "public"."user"."updated_at" IS '更新时间';
COMMENT ON COLUMN "public"."user"."deleted_at" IS '软删除时间';

-- 租户信息
COMMENT ON COLUMN "public"."user"."tenant_id" IS '租户ID';

-- 账户信息
COMMENT ON COLUMN "public"."user"."username" IS '用户名（唯一）';
COMMENT ON COLUMN "public"."user"."email" IS '邮箱地址';
COMMENT ON COLUMN "public"."user"."mobile" IS '手机号码';
COMMENT ON COLUMN "public"."user"."password_hash" IS '密码哈希';

-- 个人信息
COMMENT ON COLUMN "public"."user"."real_name" IS '真实姓名';
COMMENT ON COLUMN "public"."user"."nick_name" IS '用户昵称';
COMMENT ON COLUMN "public"."user"."avatar_url" IS '头像URL';
COMMENT ON COLUMN "public"."user"."gender" IS '性别：0-未知 1-男 2-女';
COMMENT ON COLUMN "public"."user"."birthday" IS '出生日期';
COMMENT ON COLUMN "public"."user"."id_card" IS '身份证号';

-- 联系信息
COMMENT ON COLUMN "public"."user"."country" IS '国家';
COMMENT ON COLUMN "public"."user"."province" IS '省份';
COMMENT ON COLUMN "public"."user"."city" IS '城市';
COMMENT ON COLUMN "public"."user"."district" IS '区县';
COMMENT ON COLUMN "public"."user"."address" IS '详细地址';
COMMENT ON COLUMN "public"."user"."postal_code" IS '邮政编码';

-- 账户状态
COMMENT ON COLUMN "public"."user"."status" IS '账户状态：0-禁用 1-正常 2-锁定';
COMMENT ON COLUMN "public"."user"."is_email_verified" IS '邮箱是否验证';
COMMENT ON COLUMN "public"."user"."is_mobile_verified" IS '手机是否验证';

-- 系统信息
COMMENT ON COLUMN "public"."user"."last_login_time" IS '最后登录时间';
COMMENT ON COLUMN "public"."user"."last_login_ip" IS '最后登录IP';
COMMENT ON COLUMN "public"."user"."login_count" IS '登录次数';

-- 扩展字段
COMMENT ON COLUMN "public"."user"."source" IS '注册来源：web, app, wechat等';
COMMENT ON COLUMN "public"."user"."invite_code" IS '邀请码';
COMMENT ON COLUMN "public"."user"."referrer_id" IS '推荐人ID';

-- 备注
COMMENT ON COLUMN "public"."user"."remark" IS '备注信息';

-- ================================
-- 用户表创建完成
-- ================================
