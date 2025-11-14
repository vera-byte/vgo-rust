-- ================================
-- 修复时间时区问题迁移脚本
-- ================================

-- 1. 更新公共函数使用NOW()确保时区一致性
CREATE OR REPLACE FUNCTION public.update_update_time()
RETURNS TRIGGER AS $$
BEGIN
    NEW."update_time" = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 更新user_wx表的默认值
ALTER TABLE "public"."user_wx" 
ALTER COLUMN "create_time" SET DEFAULT NOW(),
ALTER COLUMN "update_time" SET DEFAULT NOW();

-- 3. 更新现有数据的时间字段（如果有数据）
-- UPDATE "public"."user_wx" SET 
-- "create_time" = "create_time" AT TIME ZONE 'UTC' AT TIME ZONE 'Asia/Shanghai',
-- "update_time" = "update_time" AT TIME ZONE 'UTC' AT TIME ZONE 'Asia/Shanghai'
-- WHERE "create_time" IS NOT NULL;

-- 4. 设置会话时区建议
COMMENT ON TABLE "public"."user_wx" IS '微信用户信息表';

-- 修复完成