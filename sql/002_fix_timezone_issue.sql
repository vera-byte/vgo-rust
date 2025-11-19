-- ================================
-- 修复时间时区问题迁移脚本
-- ================================

-- 1. 统一 updated_at 字段的触发器逻辑
-- Unify trigger to update NEW.updated_at
CREATE OR REPLACE FUNCTION public.update_update_time()
RETURNS TRIGGER AS $$
BEGIN
    NEW."updated_at" = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 更新 user_wx 表的默认值（统一为 created_at/updated_at）
ALTER TABLE "public"."user_wx"
ALTER COLUMN "created_at" SET DEFAULT NOW(),
ALTER COLUMN "updated_at" SET DEFAULT NOW();

-- 3. 更新现有数据的时间字段（如果需要）
-- UPDATE "public"."user_wx" SET
-- "created_at" = "created_at" AT TIME ZONE 'UTC' AT TIME ZONE 'Asia/Shanghai',
-- "updated_at" = "updated_at" AT TIME ZONE 'UTC' AT TIME ZONE 'Asia/Shanghai'
-- WHERE "created_at" IS NOT NULL;

-- 4. 设置会话时区建议
COMMENT ON TABLE "public"."user_wx" IS '微信用户信息表';

-- 修复完成
