const std = @import("std");

pub const Mood = enum { happy, sad };

pub const Greeter = struct {
    name: []const u8,

    pub fn greet(self: Greeter) []const u8 {
        return self.name;
    }

    fn secret(self: Greeter) void {
        _ = self;
    }
};

pub fn topLevelGreeting(g: Greeter) []const u8 {
    return g.greet();
}

fn internalHelper() void {}
