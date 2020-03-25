// Automatically generated from files in pathfinder/shaders/. Do not edit!
#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

struct main0_out
{
    float2 vTexCoord [[user(locn0)]];
    float4 gl_Position [[position]];
};

struct main0_in
{
    float2 aPosition [[attribute(0)]];
};

vertex main0_out main0(main0_in in [[stage_in]])
{
    main0_out out = {};
    out.vTexCoord = in.aPosition;
    out.gl_Position = float4(mix(in.aPosition, float2(-1.0), float2(1.0)), 0.0, 1.0);
    return out;
}

