const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 8, std140) uniform _U_RimBloomCalc {
    vec4 gRimBloomCalcWork[2];
}U_RimBloomCalc;
layout(binding = 7, std140) uniform _U_VolTexCalc {
    vec4 gVolTexCalcWork[10];
}U_VolTexCalc;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[3];
    vec4 gWrkCol;
}U_Mate;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 0) uniform sampler3D volTex0;
layout(binding = 1) uniform sampler2D s2;
layout(binding = 2) uniform sampler2D s1;
layout(binding = 3) uniform sampler2D s0;
layout(binding = 4) uniform sampler2D s4;
layout(binding = 5) uniform sampler2D s3;
layout(binding = 6) uniform sampler2D s5;
layout(binding = 7) uniform sampler2D s6;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 4) in vec4 in_attr4;
layout(location = 5) in vec4 in_attr5;
layout(location = 6) in vec4 in_attr6;
layout(location = 7) in vec4 in_attr7;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
void main() {
    bool temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    precise float temp_6;
    precise float temp_7;
    precise float temp_8;
    precise float temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise float temp_15;
    precise float temp_16;
    precise float temp_17;
    precise float temp_18;
    precise float temp_19;
    precise float temp_20;
    precise float temp_21;
    precise float temp_22;
    precise float temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    precise float temp_27;
    precise float temp_28;
    precise float temp_29;
    precise float temp_30;
    bool temp_31;
    precise float temp_32;
    precise float temp_33;
    precise vec3 temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise vec3 temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise vec3 temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise vec2 temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    precise float temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    precise float temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    precise float temp_66;
    precise float temp_67;
    precise float temp_68;
    precise float temp_69;
    precise float temp_70;
    precise float temp_71;
    precise float temp_72;
    precise float temp_73;
    precise float temp_74;
    precise float temp_75;
    precise float temp_76;
    precise float temp_77;
    precise float temp_78;
    precise float temp_79;
    precise float temp_80;
    precise float temp_81;
    precise float temp_82;
    precise float temp_83;
    precise float temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    precise float temp_110;
    precise float temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    precise float temp_115;
    precise float temp_116;
    precise float temp_117;
    precise float temp_118;
    precise float temp_119;
    precise float temp_120;
    precise float temp_121;
    precise float temp_122;
    precise float temp_123;
    precise float temp_124;
    precise float temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    precise float temp_129;
    precise float temp_130;
    precise float temp_131;
    precise float temp_132;
    precise float temp_133;
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise float temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise float temp_150;
    precise float temp_151;
    precise float temp_152;
    precise float temp_153;
    precise float temp_154;
    precise float temp_155;
    precise float temp_156;
    precise float temp_157;
    precise float temp_158;
    precise float temp_159;
    precise float temp_160;
    precise float temp_161;
    precise float temp_162;
    precise float temp_163;
    precise float temp_164;
    precise float temp_165;
    precise float temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    precise float temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    precise float temp_175;
    precise float temp_176;
    precise float temp_177;
    precise float temp_178;
    precise float temp_179;
    precise float temp_180;
    precise float temp_181;
    precise float temp_182;
    precise float temp_183;
    precise float temp_184;
    precise float temp_185;
    precise float temp_186;
    precise float temp_187;
    precise float temp_188;
    int temp_189;
    int temp_190;
    precise float temp_191;
    precise float temp_192;
    precise float temp_193;
    precise float temp_194;
    int temp_195;
    precise float temp_196;
    precise float temp_197;
    precise float temp_198;
    precise float temp_199;
    precise float temp_200;
    int temp_201;
    precise float temp_202;
    precise float temp_203;
    precise float temp_204;
    bool temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    precise float temp_209;
    precise float temp_210;
    precise float temp_211;
    precise float temp_212;
    precise float temp_213;
    int temp_214;
    precise float temp_215;
    precise float temp_216;
    precise float temp_217;
    precise float temp_218;
    int temp_219;
    precise float temp_220;
    precise float temp_221;
    precise float temp_222;
    precise float temp_223;
    precise float temp_224;
    precise float temp_225;
    bool temp_226;
    bool temp_227;
    precise float temp_228;
    int temp_229;
    precise float temp_230;
    int temp_231;
    int temp_232;
    int temp_233;
    int temp_234;
    int temp_235;
    int temp_236;
    int temp_237;
    int temp_238;
    int temp_239;
    int temp_240;
    int temp_241;
    int temp_242;
    int temp_243;
    int temp_244;
    precise float temp_245;
    precise float temp_246;
    int temp_247;
    int temp_248;
    int temp_249;
    precise float temp_250;
    bool temp_251;
    precise float temp_252;
    precise float temp_253;
    precise float temp_254;
    precise float temp_255;
    precise float temp_256;
    precise float temp_257;
    precise float temp_258;
    precise float temp_259;
    precise float temp_260;
    precise float temp_261;
    precise float temp_262;
    precise float temp_263;
    precise float temp_264;
    precise float temp_265;
    precise float temp_266;
    precise float temp_267;
    precise float temp_268;
    precise float temp_269;
    bool temp_270;
    int temp_271;
    int temp_272;
    int temp_273;
    int temp_274;
    bool temp_275;
    int temp_276;
    int temp_277;
    int temp_278;
    int temp_279;
    precise float temp_280;
    precise float temp_281;
    precise float temp_282;
    bool temp_283;
    bool temp_284;
    bool temp_285;
    int temp_286;
    int temp_287;
    int temp_288;
    int temp_289;
    int temp_290;
    int temp_291;
    int temp_292;
    int temp_293;
    precise float temp_294;
    precise float temp_295;
    precise float temp_296;
    bool temp_297;
    precise float temp_298;
    precise float temp_299;
    precise float temp_300;
    precise float temp_301;
    precise float temp_302;
    precise float temp_303;
    precise float temp_304;
    precise float temp_305;
    precise float temp_306;
    precise float temp_307;
    precise float temp_308;
    precise float temp_309;
    precise float temp_310;
    precise float temp_311;
    precise float temp_312;
    precise float temp_313;
    precise float temp_314;
    precise float temp_315;
    bool temp_316;
    int temp_317;
    int temp_318;
    int temp_319;
    int temp_320;
    bool temp_321;
    int temp_322;
    int temp_323;
    int temp_324;
    int temp_325;
    precise float temp_326;
    precise float temp_327;
    precise float temp_328;
    bool temp_329;
    bool temp_330;
    bool temp_331;
    int temp_332;
    int temp_333;
    int temp_334;
    int temp_335;
    int temp_336;
    int temp_337;
    int temp_338;
    int temp_339;
    precise float temp_340;
    precise float temp_341;
    precise float temp_342;
    bool temp_343;
    precise float temp_344;
    precise float temp_345;
    precise float temp_346;
    precise float temp_347;
    precise float temp_348;
    precise float temp_349;
    precise float temp_350;
    precise float temp_351;
    precise float temp_352;
    precise float temp_353;
    precise float temp_354;
    precise float temp_355;
    precise float temp_356;
    precise float temp_357;
    precise float temp_358;
    precise float temp_359;
    precise float temp_360;
    precise float temp_361;
    bool temp_362;
    int temp_363;
    int temp_364;
    int temp_365;
    int temp_366;
    bool temp_367;
    int temp_368;
    int temp_369;
    int temp_370;
    int temp_371;
    precise float temp_372;
    precise float temp_373;
    precise float temp_374;
    bool temp_375;
    bool temp_376;
    bool temp_377;
    int temp_378;
    int temp_379;
    int temp_380;
    int temp_381;
    int temp_382;
    int temp_383;
    int temp_384;
    int temp_385;
    precise float temp_386;
    precise float temp_387;
    precise float temp_388;
    bool temp_389;
    precise float temp_390;
    precise float temp_391;
    precise float temp_392;
    precise float temp_393;
    precise float temp_394;
    precise float temp_395;
    precise float temp_396;
    precise float temp_397;
    precise float temp_398;
    precise float temp_399;
    precise float temp_400;
    precise float temp_401;
    precise float temp_402;
    precise float temp_403;
    precise float temp_404;
    precise float temp_405;
    precise float temp_406;
    precise float temp_407;
    bool temp_408;
    int temp_409;
    int temp_410;
    int temp_411;
    int temp_412;
    bool temp_413;
    int temp_414;
    int temp_415;
    int temp_416;
    int temp_417;
    precise float temp_418;
    precise float temp_419;
    precise float temp_420;
    bool temp_421;
    bool temp_422;
    int temp_423;
    int temp_424;
    int temp_425;
    int temp_426;
    precise float temp_427;
    precise float temp_428;
    bool temp_429;
    precise float temp_430;
    precise float temp_431;
    precise float temp_432;
    precise float temp_433;
    precise float temp_434;
    precise float temp_435;
    precise float temp_436;
    precise float temp_437;
    precise float temp_438;
    precise float temp_439;
    precise float temp_440;
    precise float temp_441;
    precise float temp_442;
    precise float temp_443;
    precise float temp_444;
    precise float temp_445;
    precise float temp_446;
    precise float temp_447;
    int temp_448;
    int temp_449;
    int temp_450;
    int temp_451;
    precise float temp_452;
    precise float temp_453;
    precise float temp_454;
    precise float temp_455;
    precise float temp_456;
    precise float temp_457;
    precise float temp_458;
    precise float temp_459;
    precise float temp_460;
    precise float temp_461;
    precise float temp_462;
    precise float temp_463;
    precise float temp_464;
    bool temp_465;
    precise float temp_466;
    bool temp_467;
    precise float temp_468;
    precise float temp_469;
    precise float temp_470;
    bool temp_471;
    precise float temp_472;
    precise float temp_473;
    precise float temp_474;
    precise float temp_475;
    bool temp_476;
    precise float temp_477;
    precise float temp_478;
    precise float temp_479;
    precise float temp_480;
    precise float temp_481;
    precise float temp_482;
    precise float temp_483;
    precise float temp_484;
    precise float temp_485;
    precise float temp_486;
    precise float temp_487;
    precise float temp_488;
    temp_0 = 0. < U_RimBloomCalc.gRimBloomCalcWork[1].z;
    temp_1 = in_attr7.z;
    temp_2 = in_attr7.x;
    temp_3 = in_attr7.y;
    temp_4 = temp_1 * U_VolTexCalc.gVolTexCalcWork[0].z;
    temp_5 = temp_2 * U_VolTexCalc.gVolTexCalcWork[0].x;
    temp_6 = temp_3 * U_VolTexCalc.gVolTexCalcWork[0].y;
    temp_7 = texture(volTex0, vec3(temp_5, temp_6, temp_4)).x;
    temp_8 = 0. - U_VolTexCalc.gVolTexCalcWork[1].x;
    temp_9 = temp_2 + temp_8;
    temp_10 = in_attr4.x;
    temp_11 = 0. - U_VolTexCalc.gVolTexCalcWork[1].y;
    temp_12 = temp_3 + temp_11;
    temp_13 = in_attr4.y;
    temp_14 = temp_9 * temp_9;
    temp_15 = 0. - U_VolTexCalc.gVolTexCalcWork[1].z;
    temp_16 = temp_1 + temp_15;
    temp_17 = fma(temp_12, temp_12, temp_14);
    temp_18 = fma(temp_16, temp_16, temp_17);
    temp_19 = sqrt(temp_18);
    temp_20 = 1. / U_VolTexCalc.gVolTexCalcWork[4].z;
    temp_21 = U_VolTexCalc.gVolTexCalcWork[4].w + U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_22 = min(temp_19, U_VolTexCalc.gVolTexCalcWork[4].z);
    temp_23 = in_attr4.z;
    temp_24 = fma(temp_21, U_VolTexCalc.gVolTexCalcWork[0].w, U_VolTexCalc.gVolTexCalcWork[0].w);
    temp_25 = temp_22 * temp_20;
    temp_26 = in_attr4.w;
    temp_27 = 0. - U_VolTexCalc.gVolTexCalcWork[4].w;
    temp_28 = fma(temp_25, temp_27, temp_24);
    temp_29 = 0. - U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_30 = temp_28 + temp_29;
    temp_31 = temp_7 < temp_30;
    temp_32 = temp_30;
    if (temp_31) {
        discard;
    }
    temp_33 = texture(s2, vec2(temp_10, temp_13)).x;
    temp_34 = texture(s1, vec2(temp_10, temp_13)).xyz;
    temp_35 = temp_34.x;
    temp_36 = temp_34.y;
    temp_37 = temp_34.z;
    temp_38 = texture(s0, vec2(temp_10, temp_13)).xyz;
    temp_39 = temp_38.x;
    temp_40 = temp_38.y;
    temp_41 = temp_38.z;
    temp_42 = texture(s4, vec2(temp_23, temp_26)).x;
    temp_43 = texture(s3, vec2(temp_23, temp_26)).xyz;
    temp_44 = temp_43.x;
    temp_45 = temp_43.y;
    temp_46 = temp_43.z;
    temp_47 = texture(s5, vec2(temp_10, temp_13)).xy;
    temp_48 = temp_47.x;
    temp_49 = temp_47.y;
    temp_50 = texture(s6, vec2(temp_10, temp_13)).y;
    temp_51 = in_attr1.x;
    temp_52 = in_attr1.y;
    temp_53 = in_attr1.z;
    temp_54 = temp_33 * U_Mate.gWrkFl4[0].x;
    temp_55 = 0. - temp_39;
    temp_56 = temp_35 + temp_55;
    temp_57 = 0. - temp_40;
    temp_58 = temp_36 + temp_57;
    temp_59 = fma(temp_56, temp_54, temp_39);
    temp_60 = temp_51 * temp_51;
    temp_61 = fma(temp_58, temp_54, temp_40);
    temp_62 = in_attr0.y;
    temp_63 = fma(temp_52, temp_52, temp_60);
    temp_64 = 0. - temp_41;
    temp_65 = temp_37 + temp_64;
    temp_66 = fma(temp_53, temp_53, temp_63);
    temp_67 = fma(temp_65, temp_54, temp_41);
    temp_68 = in_attr0.x;
    temp_69 = inversesqrt(temp_66);
    temp_70 = temp_51 * temp_69;
    temp_71 = in_attr0.z;
    temp_72 = temp_68 * temp_68;
    temp_73 = fma(temp_62, temp_62, temp_72);
    temp_74 = fma(temp_71, temp_71, temp_73);
    temp_75 = inversesqrt(temp_74);
    temp_76 = 0. - temp_59;
    temp_77 = temp_76 + temp_44;
    temp_78 = temp_68 * temp_75;
    temp_79 = fma(temp_48, 2., -1.);
    temp_80 = fma(temp_49, 2., -1.);
    temp_81 = temp_79 * temp_79;
    temp_82 = fma(temp_77, temp_42, temp_59);
    temp_83 = fma(temp_80, temp_80, temp_81);
    temp_84 = 0. - temp_83;
    temp_85 = temp_84 + 1.;
    temp_86 = sqrt(temp_85);
    temp_87 = temp_52 * temp_69;
    temp_88 = temp_53 * temp_69;
    temp_89 = temp_79 * temp_70;
    temp_90 = in_attr2.x;
    temp_91 = temp_62 * temp_75;
    temp_92 = temp_71 * temp_75;
    temp_93 = temp_79 * temp_87;
    temp_94 = temp_79 * temp_88;
    temp_95 = in_attr2.y;
    temp_96 = max(0., temp_86);
    temp_97 = fma(temp_91, temp_96, temp_93);
    temp_98 = in_attr2.z;
    temp_99 = fma(temp_92, temp_96, temp_94);
    temp_100 = temp_90 * temp_90;
    temp_101 = fma(temp_95, temp_95, temp_100);
    temp_102 = fma(temp_78, temp_96, temp_89);
    temp_103 = fma(temp_98, temp_98, temp_101);
    temp_104 = in_attr3.z;
    temp_105 = 0. - temp_67;
    temp_106 = temp_105 + temp_46;
    temp_107 = inversesqrt(temp_103);
    temp_108 = temp_90 * temp_107;
    temp_109 = in_attr3.y;
    temp_110 = temp_95 * temp_107;
    temp_111 = temp_98 * temp_107;
    temp_112 = in_attr3.x;
    temp_113 = fma(temp_80, temp_108, temp_102);
    temp_114 = fma(temp_80, temp_110, temp_97);
    temp_115 = fma(temp_80, temp_111, temp_99);
    temp_116 = temp_113 * temp_113;
    temp_117 = temp_112 * temp_112;
    temp_118 = fma(temp_109, temp_109, temp_117);
    temp_119 = fma(temp_104, temp_104, temp_118);
    temp_120 = inversesqrt(temp_119);
    temp_121 = fma(temp_114, temp_114, temp_116);
    temp_122 = fma(temp_115, temp_115, temp_121);
    temp_123 = inversesqrt(temp_122);
    temp_124 = temp_112 * temp_120;
    temp_125 = in_attr6.w;
    temp_126 = temp_109 * temp_120;
    temp_127 = temp_113 * temp_123;
    temp_128 = temp_114 * temp_123;
    temp_129 = temp_115 * temp_123;
    temp_130 = in_attr6.x;
    temp_131 = 0. - temp_127;
    temp_132 = temp_124 * temp_131;
    temp_133 = in_attr6.y;
    temp_134 = 0. - temp_128;
    temp_135 = fma(temp_126, temp_134, temp_132);
    temp_136 = in_attr5.w;
    temp_137 = temp_104 * temp_120;
    temp_138 = 1. / temp_125;
    temp_139 = temp_130 * temp_138;
    temp_140 = 1. / temp_136;
    temp_141 = temp_133 * temp_138;
    temp_142 = in_attr5.y;
    temp_143 = 0. - temp_129;
    temp_144 = fma(temp_137, temp_143, temp_135);
    temp_145 = in_attr5.x;
    temp_146 = abs(temp_144);
    temp_147 = 0. - temp_146;
    temp_148 = temp_147 + 1.;
    temp_149 = log2(temp_148);
    temp_150 = U_Mate.gWrkFl4[0].y * 5.;
    temp_151 = 0. - temp_141;
    temp_152 = fma(temp_142, temp_140, temp_151);
    temp_153 = temp_141;
    temp_154 = temp_140;
    temp_155 = temp_139;
    temp_156 = temp_45;
    if (temp_0) {
        temp_153 = U_RimBloomCalc.gRimBloomCalcWork[1].x;
    }
    temp_157 = temp_153;
    temp_158 = 0. - temp_139;
    temp_159 = fma(temp_145, temp_140, temp_158);
    temp_160 = temp_157;
    if (temp_0) {
        temp_161 = temp_157 * 10.;
        temp_160 = temp_161;
    }
    temp_162 = temp_160;
    temp_163 = temp_150 * temp_149;
    if (temp_0) {
        temp_164 = temp_162 * temp_149;
        temp_154 = temp_164;
    }
    temp_165 = temp_154;
    temp_166 = 0. - temp_61;
    temp_167 = temp_166 + temp_45;
    temp_168 = temp_163;
    if (temp_0) {
        temp_155 = temp_165;
    }
    temp_169 = temp_155;
    temp_170 = exp2(temp_163);
    temp_171 = 0. - temp_82;
    temp_172 = temp_171 + U_Mate.gWrkCol.x;
    temp_173 = temp_170;
    temp_174 = temp_169;
    if (temp_0) {
        temp_175 = exp2(temp_169);
        temp_156 = temp_175;
    }
    temp_176 = temp_156;
    temp_177 = fma(temp_167, temp_42, temp_61);
    temp_178 = fma(temp_106, temp_42, temp_67);
    temp_179 = temp_33 * U_Mate.gWrkFl4[0].w;
    temp_180 = 0. - temp_177;
    temp_181 = temp_180 + U_Mate.gWrkCol.y;
    temp_182 = temp_179 * U_Mate.gWrkFl4[0].z;
    temp_183 = fma(temp_172, temp_170, temp_82);
    temp_184 = 0. - temp_178;
    temp_185 = temp_184 + U_Mate.gWrkCol.z;
    temp_186 = temp_176;
    temp_187 = temp_177;
    temp_188 = temp_178;
    temp_189 = floatBitsToInt(temp_182);
    temp_190 = floatBitsToInt(temp_183);
    if (temp_0) {
        temp_191 = temp_176 * U_RimBloomCalc.gRimBloomCalcWork[1].z;
        temp_186 = temp_191;
    }
    temp_192 = temp_186;
    temp_193 = temp_30 + U_VolTexCalc.gVolTexCalcWork[4].y;
    temp_194 = fma(temp_181, temp_170, temp_177);
    temp_195 = floatBitsToInt(temp_194);
    if (temp_0) {
        temp_196 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_197 = fma(temp_183, temp_196, temp_183);
        temp_187 = temp_197;
    }
    temp_198 = temp_187;
    temp_199 = fma(temp_185, temp_170, temp_178);
    temp_200 = temp_198;
    temp_201 = floatBitsToInt(temp_199);
    if (temp_0) {
        temp_202 = 0. - temp_192;
        temp_203 = fma(temp_182, temp_202, temp_192);
        temp_173 = temp_203;
    }
    temp_204 = temp_173;
    temp_205 = temp_7 < temp_193;
    temp_206 = temp_204;
    if (temp_0) {
        temp_207 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_208 = fma(temp_194, temp_207, temp_194);
        temp_188 = temp_208;
    }
    temp_209 = temp_188;
    if (temp_0) {
        temp_210 = 0. - temp_198;
        temp_211 = temp_210 + U_RimBloomCalc.gRimBloomCalcWork[0].x;
        temp_168 = temp_211;
    }
    temp_212 = temp_168;
    if (temp_0) {
        temp_213 = temp_182 + temp_204;
        temp_189 = floatBitsToInt(temp_213);
    }
    temp_214 = temp_189;
    if (temp_0) {
        temp_215 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_216 = fma(temp_199, temp_215, temp_199);
        temp_206 = temp_216;
    }
    temp_217 = temp_206;
    if (temp_0) {
        temp_218 = fma(temp_192, temp_212, temp_198);
        temp_190 = floatBitsToInt(temp_218);
    }
    temp_219 = temp_190;
    if (temp_0) {
        temp_220 = 0. - temp_209;
        temp_221 = temp_220 + U_RimBloomCalc.gRimBloomCalcWork[0].y;
        temp_200 = temp_221;
    }
    temp_222 = temp_200;
    if (temp_0) {
        temp_223 = 0. - temp_217;
        temp_224 = temp_223 + U_RimBloomCalc.gRimBloomCalcWork[0].z;
        temp_174 = temp_224;
    }
    temp_225 = temp_174;
    temp_226 = !temp_205;
    temp_227 = temp_226;
    if (temp_0) {
        temp_228 = fma(temp_192, temp_222, temp_209);
        temp_195 = floatBitsToInt(temp_228);
    }
    temp_229 = temp_195;
    if (temp_0) {
        temp_230 = fma(temp_192, temp_225, temp_217);
        temp_201 = floatBitsToInt(temp_230);
    }
    temp_231 = temp_201;
    temp_232 = temp_214;
    temp_233 = temp_219;
    if (temp_205) {
        temp_232 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].w);
    }
    temp_234 = temp_232;
    temp_235 = temp_229;
    temp_236 = temp_234;
    temp_237 = temp_234;
    if (temp_205) {
        temp_233 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].x);
    }
    temp_238 = temp_233;
    temp_239 = temp_231;
    temp_240 = temp_238;
    temp_241 = temp_238;
    if (temp_205) {
        temp_235 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].y);
    }
    temp_242 = temp_235;
    temp_243 = temp_242;
    temp_244 = temp_242;
    if (temp_226) {
        temp_32 = temp_193;
    }
    temp_245 = temp_32;
    temp_246 = temp_245;
    if (temp_205) {
        temp_239 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].z);
    }
    temp_247 = temp_239;
    temp_248 = temp_247;
    temp_249 = temp_247;
    if (temp_226) {
        temp_250 = temp_245 + U_VolTexCalc.gVolTexCalcWork[4].x;
        temp_251 = temp_7 < temp_250;
        if (temp_251) {
            temp_252 = 0. - temp_245;
            temp_253 = temp_252 + temp_250;
            temp_254 = 1. / temp_253;
            temp_255 = 0. - temp_245;
            temp_256 = temp_255 + temp_7;
            temp_257 = 0. - U_VolTexCalc.gVolTexCalcWork[9].x;
            temp_258 = U_VolTexCalc.gVolTexCalcWork[8].x + temp_257;
            temp_259 = 0. - U_VolTexCalc.gVolTexCalcWork[9].y;
            temp_260 = U_VolTexCalc.gVolTexCalcWork[8].y + temp_259;
            temp_261 = 0. - U_VolTexCalc.gVolTexCalcWork[9].z;
            temp_262 = U_VolTexCalc.gVolTexCalcWork[8].z + temp_261;
            temp_263 = temp_256 * temp_254;
            temp_264 = 0. - U_VolTexCalc.gVolTexCalcWork[9].w;
            temp_265 = U_VolTexCalc.gVolTexCalcWork[8].w + temp_264;
            temp_266 = fma(temp_258, temp_263, U_VolTexCalc.gVolTexCalcWork[9].x);
            temp_267 = fma(temp_260, temp_263, U_VolTexCalc.gVolTexCalcWork[9].y);
            temp_268 = fma(temp_262, temp_263, U_VolTexCalc.gVolTexCalcWork[9].z);
            temp_269 = fma(temp_265, temp_263, U_VolTexCalc.gVolTexCalcWork[9].w);
            temp_227 = false;
            temp_243 = floatBitsToInt(temp_267);
            temp_248 = floatBitsToInt(temp_268);
            temp_240 = floatBitsToInt(temp_266);
            temp_236 = floatBitsToInt(temp_269);
        }
        temp_270 = temp_227;
        temp_271 = temp_243;
        temp_272 = temp_248;
        temp_273 = temp_240;
        temp_274 = temp_236;
        temp_275 = temp_270;
        temp_276 = temp_273;
        temp_277 = temp_271;
        temp_278 = temp_272;
        temp_279 = temp_274;
        temp_244 = temp_271;
        temp_249 = temp_272;
        temp_241 = temp_273;
        temp_237 = temp_274;
        if (temp_270) {
            temp_246 = temp_250;
        }
        temp_280 = temp_246;
        temp_281 = temp_280;
        if (temp_270) {
            temp_282 = temp_280 + U_VolTexCalc.gVolTexCalcWork[3].w;
            temp_283 = temp_7 < temp_282;
            if (temp_283) {
                temp_275 = false;
            }
            temp_284 = temp_275;
            temp_285 = temp_284;
            if (temp_283) {
                temp_276 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].x);
            }
            temp_286 = temp_276;
            temp_287 = temp_286;
            temp_241 = temp_286;
            if (temp_283) {
                temp_277 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].y);
            }
            temp_288 = temp_277;
            temp_289 = temp_288;
            temp_244 = temp_288;
            if (temp_283) {
                temp_278 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].z);
            }
            temp_290 = temp_278;
            temp_291 = temp_290;
            temp_249 = temp_290;
            if (temp_283) {
                temp_279 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].w);
            }
            temp_292 = temp_279;
            temp_293 = temp_292;
            temp_237 = temp_292;
            if (temp_284) {
                temp_281 = temp_282;
            }
            temp_294 = temp_281;
            temp_295 = temp_294;
            if (temp_284) {
                temp_296 = temp_294 + U_VolTexCalc.gVolTexCalcWork[3].z;
                temp_297 = temp_7 < temp_296;
                if (temp_297) {
                    temp_298 = 0. - temp_294;
                    temp_299 = temp_298 + temp_296;
                    temp_300 = 1. / temp_299;
                    temp_301 = 0. - temp_294;
                    temp_302 = temp_301 + temp_7;
                    temp_303 = 0. - U_VolTexCalc.gVolTexCalcWork[8].x;
                    temp_304 = U_VolTexCalc.gVolTexCalcWork[7].x + temp_303;
                    temp_305 = 0. - U_VolTexCalc.gVolTexCalcWork[8].y;
                    temp_306 = U_VolTexCalc.gVolTexCalcWork[7].y + temp_305;
                    temp_307 = 0. - U_VolTexCalc.gVolTexCalcWork[8].z;
                    temp_308 = U_VolTexCalc.gVolTexCalcWork[7].z + temp_307;
                    temp_309 = 0. - U_VolTexCalc.gVolTexCalcWork[8].w;
                    temp_310 = U_VolTexCalc.gVolTexCalcWork[7].w + temp_309;
                    temp_311 = temp_302 * temp_300;
                    temp_312 = fma(temp_304, temp_311, U_VolTexCalc.gVolTexCalcWork[8].x);
                    temp_313 = fma(temp_306, temp_311, U_VolTexCalc.gVolTexCalcWork[8].y);
                    temp_314 = fma(temp_308, temp_311, U_VolTexCalc.gVolTexCalcWork[8].z);
                    temp_315 = fma(temp_310, temp_311, U_VolTexCalc.gVolTexCalcWork[8].w);
                    temp_285 = false;
                    temp_289 = floatBitsToInt(temp_313);
                    temp_291 = floatBitsToInt(temp_314);
                    temp_287 = floatBitsToInt(temp_312);
                    temp_293 = floatBitsToInt(temp_315);
                }
                temp_316 = temp_285;
                temp_317 = temp_289;
                temp_318 = temp_291;
                temp_319 = temp_287;
                temp_320 = temp_293;
                temp_321 = temp_316;
                temp_322 = temp_319;
                temp_323 = temp_317;
                temp_324 = temp_318;
                temp_325 = temp_320;
                temp_244 = temp_317;
                temp_249 = temp_318;
                temp_241 = temp_319;
                temp_237 = temp_320;
                if (temp_316) {
                    temp_295 = temp_296;
                }
                temp_326 = temp_295;
                temp_327 = temp_326;
                if (temp_316) {
                    temp_328 = temp_326 + U_VolTexCalc.gVolTexCalcWork[3].y;
                    temp_329 = temp_7 < temp_328;
                    if (temp_329) {
                        temp_321 = false;
                    }
                    temp_330 = temp_321;
                    temp_331 = temp_330;
                    if (temp_329) {
                        temp_322 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].x);
                    }
                    temp_332 = temp_322;
                    temp_333 = temp_332;
                    temp_241 = temp_332;
                    if (temp_329) {
                        temp_323 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].y);
                    }
                    temp_334 = temp_323;
                    temp_335 = temp_334;
                    temp_244 = temp_334;
                    if (temp_329) {
                        temp_324 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].z);
                    }
                    temp_336 = temp_324;
                    temp_337 = temp_336;
                    temp_249 = temp_336;
                    if (temp_329) {
                        temp_325 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].w);
                    }
                    temp_338 = temp_325;
                    temp_339 = temp_338;
                    temp_237 = temp_338;
                    if (temp_330) {
                        temp_327 = temp_328;
                    }
                    temp_340 = temp_327;
                    temp_341 = temp_340;
                    if (temp_330) {
                        temp_342 = temp_340 + U_VolTexCalc.gVolTexCalcWork[3].x;
                        temp_343 = temp_7 < temp_342;
                        if (temp_343) {
                            temp_344 = 0. - temp_340;
                            temp_345 = temp_344 + temp_342;
                            temp_346 = 1. / temp_345;
                            temp_347 = 0. - temp_340;
                            temp_348 = temp_347 + temp_7;
                            temp_349 = 0. - U_VolTexCalc.gVolTexCalcWork[7].x;
                            temp_350 = U_VolTexCalc.gVolTexCalcWork[6].x + temp_349;
                            temp_351 = 0. - U_VolTexCalc.gVolTexCalcWork[7].y;
                            temp_352 = U_VolTexCalc.gVolTexCalcWork[6].y + temp_351;
                            temp_353 = 0. - U_VolTexCalc.gVolTexCalcWork[7].z;
                            temp_354 = U_VolTexCalc.gVolTexCalcWork[6].z + temp_353;
                            temp_355 = 0. - U_VolTexCalc.gVolTexCalcWork[7].w;
                            temp_356 = U_VolTexCalc.gVolTexCalcWork[6].w + temp_355;
                            temp_357 = temp_348 * temp_346;
                            temp_358 = fma(temp_350, temp_357, U_VolTexCalc.gVolTexCalcWork[7].x);
                            temp_359 = fma(temp_352, temp_357, U_VolTexCalc.gVolTexCalcWork[7].y);
                            temp_360 = fma(temp_354, temp_357, U_VolTexCalc.gVolTexCalcWork[7].z);
                            temp_361 = fma(temp_356, temp_357, U_VolTexCalc.gVolTexCalcWork[7].w);
                            temp_331 = false;
                            temp_335 = floatBitsToInt(temp_359);
                            temp_337 = floatBitsToInt(temp_360);
                            temp_333 = floatBitsToInt(temp_358);
                            temp_339 = floatBitsToInt(temp_361);
                        }
                        temp_362 = temp_331;
                        temp_363 = temp_335;
                        temp_364 = temp_337;
                        temp_365 = temp_333;
                        temp_366 = temp_339;
                        temp_367 = temp_362;
                        temp_368 = temp_365;
                        temp_369 = temp_363;
                        temp_370 = temp_364;
                        temp_371 = temp_366;
                        temp_244 = temp_363;
                        temp_249 = temp_364;
                        temp_241 = temp_365;
                        temp_237 = temp_366;
                        if (temp_362) {
                            temp_341 = temp_342;
                        }
                        temp_372 = temp_341;
                        temp_373 = temp_372;
                        if (temp_362) {
                            temp_374 = temp_372 + U_VolTexCalc.gVolTexCalcWork[2].w;
                            temp_375 = temp_7 < temp_374;
                            if (temp_375) {
                                temp_367 = false;
                            }
                            temp_376 = temp_367;
                            temp_377 = temp_376;
                            if (temp_375) {
                                temp_368 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].x);
                            }
                            temp_378 = temp_368;
                            temp_379 = temp_378;
                            temp_241 = temp_378;
                            if (temp_375) {
                                temp_369 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].y);
                            }
                            temp_380 = temp_369;
                            temp_381 = temp_380;
                            temp_244 = temp_380;
                            if (temp_375) {
                                temp_370 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].z);
                            }
                            temp_382 = temp_370;
                            temp_383 = temp_382;
                            temp_249 = temp_382;
                            if (temp_375) {
                                temp_371 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].w);
                            }
                            temp_384 = temp_371;
                            temp_385 = temp_384;
                            temp_237 = temp_384;
                            if (temp_376) {
                                temp_373 = temp_374;
                            }
                            temp_386 = temp_373;
                            temp_387 = temp_386;
                            if (temp_376) {
                                temp_388 = temp_386 + U_VolTexCalc.gVolTexCalcWork[2].z;
                                temp_389 = temp_7 < temp_388;
                                if (temp_389) {
                                    temp_390 = 0. - temp_386;
                                    temp_391 = temp_390 + temp_388;
                                    temp_392 = 1. / temp_391;
                                    temp_393 = 0. - temp_386;
                                    temp_394 = temp_393 + temp_7;
                                    temp_395 = 0. - U_VolTexCalc.gVolTexCalcWork[6].x;
                                    temp_396 = U_VolTexCalc.gVolTexCalcWork[5].x + temp_395;
                                    temp_397 = 0. - U_VolTexCalc.gVolTexCalcWork[6].y;
                                    temp_398 = U_VolTexCalc.gVolTexCalcWork[5].y + temp_397;
                                    temp_399 = 0. - U_VolTexCalc.gVolTexCalcWork[6].z;
                                    temp_400 = U_VolTexCalc.gVolTexCalcWork[5].z + temp_399;
                                    temp_401 = 0. - U_VolTexCalc.gVolTexCalcWork[6].w;
                                    temp_402 = U_VolTexCalc.gVolTexCalcWork[5].w + temp_401;
                                    temp_403 = temp_394 * temp_392;
                                    temp_404 = fma(temp_396, temp_403, U_VolTexCalc.gVolTexCalcWork[6].x);
                                    temp_405 = fma(temp_398, temp_403, U_VolTexCalc.gVolTexCalcWork[6].y);
                                    temp_406 = fma(temp_400, temp_403, U_VolTexCalc.gVolTexCalcWork[6].z);
                                    temp_407 = fma(temp_402, temp_403, U_VolTexCalc.gVolTexCalcWork[6].w);
                                    temp_377 = false;
                                    temp_381 = floatBitsToInt(temp_405);
                                    temp_383 = floatBitsToInt(temp_406);
                                    temp_379 = floatBitsToInt(temp_404);
                                    temp_385 = floatBitsToInt(temp_407);
                                }
                                temp_408 = temp_377;
                                temp_409 = temp_381;
                                temp_410 = temp_383;
                                temp_411 = temp_379;
                                temp_412 = temp_385;
                                temp_413 = temp_408;
                                temp_414 = temp_411;
                                temp_415 = temp_409;
                                temp_416 = temp_410;
                                temp_417 = temp_412;
                                temp_244 = temp_409;
                                temp_249 = temp_410;
                                temp_241 = temp_411;
                                temp_237 = temp_412;
                                if (temp_408) {
                                    temp_387 = temp_388;
                                }
                                temp_418 = temp_387;
                                temp_419 = temp_418;
                                if (temp_408) {
                                    temp_420 = temp_418 + U_VolTexCalc.gVolTexCalcWork[2].y;
                                    temp_421 = temp_7 < temp_420;
                                    if (temp_421) {
                                        temp_413 = false;
                                    }
                                    temp_422 = temp_413;
                                    if (temp_421) {
                                        temp_414 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].x);
                                    }
                                    temp_423 = temp_414;
                                    temp_241 = temp_423;
                                    if (temp_421) {
                                        temp_415 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].y);
                                    }
                                    temp_424 = temp_415;
                                    temp_244 = temp_424;
                                    if (temp_421) {
                                        temp_416 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].z);
                                    }
                                    temp_425 = temp_416;
                                    temp_249 = temp_425;
                                    if (temp_421) {
                                        temp_417 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].w);
                                    }
                                    temp_426 = temp_417;
                                    temp_237 = temp_426;
                                    if (temp_422) {
                                        temp_419 = temp_420;
                                    }
                                    temp_427 = temp_419;
                                    if (temp_422) {
                                        temp_428 = temp_427 + U_VolTexCalc.gVolTexCalcWork[2].x;
                                        temp_429 = temp_7 < temp_428;
                                        if (temp_429) {
                                            temp_430 = 0. - temp_427;
                                            temp_431 = temp_430 + temp_428;
                                            temp_432 = 1. / temp_431;
                                            temp_433 = 0. - temp_427;
                                            temp_434 = temp_433 + temp_7;
                                            temp_435 = 0. - U_VolTexCalc.gVolTexCalcWork[5].x;
                                            temp_436 = intBitsToFloat(temp_219) + temp_435;
                                            temp_437 = 0. - U_VolTexCalc.gVolTexCalcWork[5].y;
                                            temp_438 = intBitsToFloat(temp_229) + temp_437;
                                            temp_439 = 0. - U_VolTexCalc.gVolTexCalcWork[5].z;
                                            temp_440 = intBitsToFloat(temp_231) + temp_439;
                                            temp_441 = 0. - U_VolTexCalc.gVolTexCalcWork[5].w;
                                            temp_442 = intBitsToFloat(temp_214) + temp_441;
                                            temp_443 = temp_434 * temp_432;
                                            temp_444 = fma(temp_436, temp_443, U_VolTexCalc.gVolTexCalcWork[5].x);
                                            temp_445 = fma(temp_438, temp_443, U_VolTexCalc.gVolTexCalcWork[5].y);
                                            temp_446 = fma(temp_440, temp_443, U_VolTexCalc.gVolTexCalcWork[5].z);
                                            temp_447 = fma(temp_442, temp_443, U_VolTexCalc.gVolTexCalcWork[5].w);
                                            temp_244 = floatBitsToInt(temp_445);
                                            temp_249 = floatBitsToInt(temp_446);
                                            temp_241 = floatBitsToInt(temp_444);
                                            temp_237 = floatBitsToInt(temp_447);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    temp_448 = temp_244;
    temp_449 = temp_249;
    temp_450 = temp_241;
    temp_451 = temp_237;
    temp_452 = temp_159 * 0.5;
    temp_453 = in_attr5.z;
    temp_454 = temp_152 * 0.5;
    temp_455 = abs(temp_452);
    temp_456 = abs(temp_454);
    temp_457 = max(temp_455, temp_456);
    temp_458 = max(temp_457, 1.);
    temp_459 = 1. / temp_458;
    temp_460 = temp_454 * temp_459;
    temp_461 = temp_452 * temp_459;
    temp_462 = temp_453 * 8.;
    temp_463 = abs(temp_461);
    temp_464 = inversesqrt(temp_463);
    temp_465 = temp_460 >= 0.;
    temp_466 = floor(temp_462);
    temp_467 = temp_461 >= 0.;
    temp_468 = abs(temp_460);
    temp_469 = inversesqrt(temp_468);
    temp_470 = 1. / temp_464;
    temp_471 = !temp_465;
    temp_472 = temp_471 ? 0. : 1.;
    temp_473 = 1. / temp_469;
    temp_474 = temp_466 * 0.003921569;
    temp_475 = floor(temp_474);
    temp_476 = !temp_467;
    temp_477 = temp_476 ? 0. : 1.;
    temp_478 = temp_472 * 0.6666667;
    temp_479 = fma(temp_129, 1000., 0.5);
    temp_480 = fma(temp_477, 0.33333334, temp_478);
    temp_481 = fma(temp_128, 0.5, 0.5);
    temp_482 = fma(temp_127, 0.5, 0.5);
    temp_483 = 0. - temp_466;
    temp_484 = temp_462 + temp_483;
    temp_485 = 0. - temp_475;
    temp_486 = temp_474 + temp_485;
    temp_487 = temp_475 * 0.003921569;
    temp_488 = temp_480 + 0.01;
    out_attr0.x = intBitsToFloat(temp_450);
    out_attr0.y = intBitsToFloat(temp_448);
    out_attr0.z = intBitsToFloat(temp_449);
    out_attr0.w = intBitsToFloat(temp_451);
    out_attr1.x = U_Mate.gWrkFl4[2].x;
    out_attr1.y = U_Mate.gWrkFl4[1].w;
    out_attr1.z = U_Mate.gWrkFl4[1].x;
    out_attr1.w = 0.008235293;
    out_attr2.x = temp_482;
    out_attr2.y = temp_481;
    out_attr2.z = temp_50;
    out_attr2.w = temp_479;
    out_attr3.x = temp_470;
    out_attr3.y = temp_473;
    out_attr3.z = 0.;
    out_attr3.w = temp_488;
    out_attr4.x = temp_484;
    out_attr4.y = temp_486;
    out_attr4.z = temp_487;
    out_attr4.w = 0.;
    return;
}
