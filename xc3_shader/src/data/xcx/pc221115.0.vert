R1.x = R5.x;
R1.y = R5.y;
R5.w = 1.0;
R0.w = R1.x * 4e-45;
R0.z = R0.w + 1.0;
R1.w = R0.w + 3e-45;
R9.w = R2.w;
PARAM0.x = R1.x;
PARAM0.y = R1.y;
PARAM0.z = R1.z;
PARAM0.z = R1.z;
temp5 = dot(vec4(R3.x, R3.y, R3.z, R3.w), vec4(R7.x, R7.y, R7.z, R7.w));
R5.x = temp5;
PV5.y = temp5;
PV5.z = temp5;
PV5.w = temp5;
R127.x = R2.z * R7.z;
temp6 = dot(vec4(R3.x, R3.y, R3.z, R3.w), vec4(R0.x, R0.y, R0.z, R0.w));
PV6.x = temp6;
R5.y = temp6;
PV6.z = temp6;
PV6.w = temp6;
R126.x = R2.z * R0.z;
temp7 = dot(vec4(R3.x, R3.y, R3.z, R3.w), vec4(R8.x, R8.y, R8.z, R8.w));
PV7.x = temp7;
PV7.y = temp7;
R5.z = temp7;
PV7.w = temp7;
R125.x = R2.z * R8.z;
temp8 = dot(vec4(KC0[5].x, KC0[5].y, KC0[5].z, KC0[5].w), vec4(R5.x, R5.y, PV7.x, R5.w));
PV8.x = temp8;
PV8.y = temp8;
R3.z = temp8;
PV8.w = temp8;
R122.x = fma(R2.y, R7.y, R127.x);
temp9 = dot(vec4(KC0[3].x, KC0[3].y, KC0[3].z, KC0[3].w), vec4(R5.x, R5.y, R5.z, R5.w));
R3.x = temp9;
PV9.y = temp9;
PV9.z = temp9;
PV9.w = temp9;
R9.x = fma(R2.x, R7.x, R122.x);
temp10 = dot(vec4(KC0[4].x, KC0[4].y, KC0[4].z, KC0[4].w), vec4(R5.x, R5.y, R5.z, R5.w));
PV10.x = temp10;
R3.y = temp10;
PV10.z = temp10;
PV10.w = temp10;
R122.x = fma(R2.y, R0.y, R126.x);
temp11 = dot(vec4(KC0[6].x, KC0[6].y, KC0[6].z, KC0[6].w), vec4(R5.x, R5.y, R5.z, R5.w));
PV11.x = temp11;
PV11.y = temp11;
PV11.z = temp11;
R3.w = temp11;
R9.y = fma(R2.x, R0.x, R122.x);
temp12 = dot(vec4(R2.x, R2.y, R125.x, -0.0), vec4(R8.x, R8.y, 1.0, 0.0));
PV12.x = temp12;
PV12.y = temp12;
R9.z = temp12;
PV12.w = temp12;
R126.x = R3.z + -KC0[23].y;
temp13 = dot(vec4(R4.x, R4.y, R4.z, -0.0), vec4(R7.x, R7.y, R7.z, 0.0));
R1.x = temp13;
PV13.y = temp13;
PV13.z = temp13;
PV13.w = temp13;
PS13 = R4.z * R0.z;
temp14 = dot(vec4(R4.x, R4.y, PS13, -0.0), vec4(R0.x, R0.y, 1.0, 0.0));
PV14.x = temp14;
R1.y = temp14;
PV14.z = temp14;
PV14.w = temp14;
R125.x = R9.z * R1.x;
temp15 = dot(vec4(R4.x, R4.y, R4.z, -0.0), vec4(R8.x, R8.y, R8.z, 0.0));
PV15.x = temp15;
PV15.y = temp15;
R1.z = temp15;
PV15.w = temp15;
PS15 = R9.x * PV14.x;
R123.x = fma(-PV15.x, R9.x, R125.x);
PV16.y = R9.y * PV15.x;
PV16.z = R6.y * R0.y;
R123.w = fma(-R1.x, R9.y, PS15);
PS16 = R6.z * R7.z;
R123.x = fma(R6.y, R7.y, PS16);
R2.y = R4.w * R123.x;
R123.z = fma(-R1.y, R9.z, PV16.y);
R2.w = R4.w * R123.w;
R122.x = fma(R6.z, R0.z, PV16.z);
R0.x = fma(R6.x, R7.x, R123.x);
R0.y = fma(R6.x, R0.x, R122.x);
PV18.z = R6.z * R8.z;
R2.x = R4.w * R123.z;
temp19 = dot(vec4(R6.x, R6.y, PV18.z, -0.0), vec4(R8.x, R8.y, 1.0, 0.0));
PV19.x = temp19;
PV19.y = temp19;
R0.z = temp19;
PV19.w = temp19;
R6.z = KC0[23].z * R126.x;
R6.x = R3.x;
R6.y = R3.y;
R6.w = R3.w;
POS0.x = R3.x;
POS0.y = R3.y;
POS0.z = R3.z;
POS0.w = R3.w;
PARAM1.x = R6.x;
PARAM1.y = R6.y;
PARAM1.z = R6.z;
PARAM1.w = R6.w;
PARAM2.x = R2.x;
PARAM2.y = R2.y;
PARAM2.w = R2.w;
PARAM2.w = R2.w;
PARAM3.x = R0.x;
PARAM3.y = R0.y;
PARAM3.z = R0.z;
PARAM3.w = R0.w;
PARAM4.x = R9.x;
PARAM4.y = R9.y;
PARAM4.z = R9.z;
PARAM4.w = R9.w;
PARAM5.x = R1.x;
PARAM5.y = R1.y;
PARAM5.z = R1.z;
PARAM5.w = R1.w;
PARAM6.x = R5.x;
PARAM6.y = R5.y;
PARAM6.z = R5.z;
PARAM6.w = R5.w;
