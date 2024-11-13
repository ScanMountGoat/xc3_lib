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
layout(binding = 7, std140) uniform _U_VolTexCalc {
    vec4 gVolTexCalcWork[10];
}U_VolTexCalc;
layout(binding = 8, std140) uniform _U_RimBloomCalc {
    vec4 gRimBloomCalcWork[2];
}U_RimBloomCalc;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[2];
}U_Mate;
layout(binding = 0) uniform sampler3D volTex0;
layout(binding = 1) uniform sampler2D s0;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 4) in vec4 in_attr4;
layout(location = 5) in vec4 in_attr5;
layout(location = 6) in vec4 in_attr6;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
void main() {
    precise float temp_0;
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
    bool temp_28;
    precise vec3 temp_29;
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    precise float temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise float temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise float temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise float temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    bool temp_54;
    precise float temp_55;
    precise float temp_56;
    int temp_57;
    precise float temp_58;
    precise float temp_59;
    bool temp_60;
    bool temp_61;
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
    int temp_89;
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
    int temp_122;
    precise float temp_123;
    bool temp_124;
    bool temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    int temp_129;
    precise float temp_130;
    bool temp_131;
    precise float temp_132;
    precise float temp_133;
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    int temp_141;
    int temp_142;
    int temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    bool temp_147;
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
    bool temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    int temp_170;
    bool temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    int temp_175;
    precise float temp_176;
    precise float temp_177;
    precise float temp_178;
    bool temp_179;
    bool temp_180;
    bool temp_181;
    precise float temp_182;
    precise float temp_183;
    precise float temp_184;
    precise float temp_185;
    precise float temp_186;
    precise float temp_187;
    int temp_188;
    int temp_189;
    precise float temp_190;
    precise float temp_191;
    precise float temp_192;
    bool temp_193;
    precise float temp_194;
    precise float temp_195;
    precise float temp_196;
    precise float temp_197;
    precise float temp_198;
    precise float temp_199;
    precise float temp_200;
    precise float temp_201;
    precise float temp_202;
    precise float temp_203;
    precise float temp_204;
    precise float temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    precise float temp_209;
    precise float temp_210;
    precise float temp_211;
    bool temp_212;
    precise float temp_213;
    precise float temp_214;
    precise float temp_215;
    int temp_216;
    bool temp_217;
    precise float temp_218;
    precise float temp_219;
    precise float temp_220;
    int temp_221;
    precise float temp_222;
    precise float temp_223;
    precise float temp_224;
    bool temp_225;
    bool temp_226;
    bool temp_227;
    precise float temp_228;
    precise float temp_229;
    precise float temp_230;
    precise float temp_231;
    precise float temp_232;
    precise float temp_233;
    int temp_234;
    int temp_235;
    precise float temp_236;
    precise float temp_237;
    precise float temp_238;
    bool temp_239;
    precise float temp_240;
    precise float temp_241;
    precise float temp_242;
    precise float temp_243;
    precise float temp_244;
    precise float temp_245;
    precise float temp_246;
    precise float temp_247;
    precise float temp_248;
    precise float temp_249;
    precise float temp_250;
    precise float temp_251;
    precise float temp_252;
    precise float temp_253;
    precise float temp_254;
    precise float temp_255;
    precise float temp_256;
    precise float temp_257;
    bool temp_258;
    precise float temp_259;
    precise float temp_260;
    precise float temp_261;
    int temp_262;
    bool temp_263;
    precise float temp_264;
    precise float temp_265;
    precise float temp_266;
    int temp_267;
    precise float temp_268;
    precise float temp_269;
    precise float temp_270;
    bool temp_271;
    bool temp_272;
    bool temp_273;
    precise float temp_274;
    precise float temp_275;
    precise float temp_276;
    precise float temp_277;
    precise float temp_278;
    precise float temp_279;
    int temp_280;
    int temp_281;
    precise float temp_282;
    precise float temp_283;
    precise float temp_284;
    bool temp_285;
    precise float temp_286;
    precise float temp_287;
    precise float temp_288;
    precise float temp_289;
    precise float temp_290;
    precise float temp_291;
    precise float temp_292;
    precise float temp_293;
    precise float temp_294;
    precise float temp_295;
    precise float temp_296;
    precise float temp_297;
    precise float temp_298;
    precise float temp_299;
    precise float temp_300;
    precise float temp_301;
    precise float temp_302;
    precise float temp_303;
    bool temp_304;
    precise float temp_305;
    precise float temp_306;
    precise float temp_307;
    int temp_308;
    bool temp_309;
    precise float temp_310;
    precise float temp_311;
    precise float temp_312;
    int temp_313;
    precise float temp_314;
    precise float temp_315;
    precise float temp_316;
    bool temp_317;
    bool temp_318;
    precise float temp_319;
    precise float temp_320;
    precise float temp_321;
    int temp_322;
    precise float temp_323;
    precise float temp_324;
    bool temp_325;
    precise float temp_326;
    precise float temp_327;
    precise float temp_328;
    precise float temp_329;
    precise float temp_330;
    precise float temp_331;
    precise float temp_332;
    precise float temp_333;
    precise float temp_334;
    precise float temp_335;
    precise float temp_336;
    precise float temp_337;
    precise float temp_338;
    precise float temp_339;
    precise float temp_340;
    precise float temp_341;
    precise float temp_342;
    precise float temp_343;
    precise float temp_344;
    precise float temp_345;
    precise float temp_346;
    int temp_347;
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
    precise float temp_363;
    bool temp_364;
    precise float temp_365;
    precise float temp_366;
    precise float temp_367;
    precise float temp_368;
    precise float temp_369;
    precise float temp_370;
    bool temp_371;
    precise float temp_372;
    precise float temp_373;
    precise float temp_374;
    precise float temp_375;
    bool temp_376;
    precise float temp_377;
    precise float temp_378;
    precise float temp_379;
    precise float temp_380;
    precise float temp_381;
    precise float temp_382;
    precise float temp_383;
    precise float temp_384;
    precise float temp_385;
    precise float temp_386;
    precise float temp_387;
    precise float temp_388;
    precise float temp_389;
    precise float temp_390;
    precise float temp_391;
    precise float temp_392;
    temp_0 = in_attr6.z;
    temp_1 = in_attr6.x;
    temp_2 = in_attr6.y;
    temp_3 = temp_0 * U_VolTexCalc.gVolTexCalcWork[0].z;
    temp_4 = temp_1 * U_VolTexCalc.gVolTexCalcWork[0].x;
    temp_5 = temp_2 * U_VolTexCalc.gVolTexCalcWork[0].y;
    temp_6 = texture(volTex0, vec3(temp_4, temp_5, temp_3)).x;
    temp_7 = 0. - U_VolTexCalc.gVolTexCalcWork[1].x;
    temp_8 = temp_1 + temp_7;
    temp_9 = 1. / U_VolTexCalc.gVolTexCalcWork[4].z;
    temp_10 = 0. - U_VolTexCalc.gVolTexCalcWork[1].y;
    temp_11 = temp_2 + temp_10;
    temp_12 = temp_8 * temp_8;
    temp_13 = 0. - U_VolTexCalc.gVolTexCalcWork[1].z;
    temp_14 = temp_0 + temp_13;
    temp_15 = fma(temp_11, temp_11, temp_12);
    temp_16 = fma(temp_14, temp_14, temp_15);
    temp_17 = sqrt(temp_16);
    temp_18 = U_VolTexCalc.gVolTexCalcWork[4].w + U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_19 = min(temp_17, U_VolTexCalc.gVolTexCalcWork[4].z);
    temp_20 = in_attr2.x;
    temp_21 = fma(temp_18, U_VolTexCalc.gVolTexCalcWork[0].w, U_VolTexCalc.gVolTexCalcWork[0].w);
    temp_22 = temp_19 * temp_9;
    temp_23 = 0. - U_VolTexCalc.gVolTexCalcWork[4].w;
    temp_24 = fma(temp_22, temp_23, temp_21);
    temp_25 = in_attr2.y;
    temp_26 = 0. - U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_27 = temp_24 + temp_26;
    temp_28 = temp_6 < temp_27;
    if (temp_28) {
        discard;
    }
    temp_29 = texture(s0, vec2(temp_20, temp_25)).xyz;
    temp_30 = temp_29.x;
    temp_31 = temp_29.y;
    temp_32 = temp_29.z;
    temp_33 = intBitsToFloat(gl_FrontFacing ? -1 : 0);
    temp_34 = in_attr0.x;
    temp_35 = in_attr0.y;
    temp_36 = in_attr1.x;
    temp_37 = in_attr0.z;
    temp_38 = in_attr1.y;
    temp_39 = in_attr5.w;
    temp_40 = in_attr1.z;
    temp_41 = in_attr4.w;
    temp_42 = in_attr5.x;
    temp_43 = in_attr5.y;
    temp_44 = in_attr4.x;
    temp_45 = float(floatBitsToInt(temp_33));
    temp_46 = temp_34 * temp_34;
    temp_47 = 1. / temp_39;
    temp_48 = fma(temp_45, -2., -1.);
    temp_49 = in_attr4.y;
    temp_50 = fma(temp_35, temp_35, temp_46);
    temp_51 = 1. / temp_41;
    temp_52 = temp_36 * temp_36;
    temp_53 = fma(temp_37, temp_37, temp_50);
    temp_54 = floatBitsToInt(temp_48) > 0;
    temp_55 = inversesqrt(temp_53);
    temp_56 = fma(temp_38, temp_38, temp_52);
    temp_57 = 0 - (temp_54 ? -1 : 0);
    temp_58 = fma(temp_40, temp_40, temp_56);
    temp_59 = inversesqrt(temp_58);
    temp_60 = 0. < U_RimBloomCalc.gRimBloomCalcWork[1].z;
    temp_61 = temp_57 == 0;
    temp_62 = temp_42 * temp_47;
    temp_63 = temp_43 * temp_47;
    temp_64 = temp_34 * temp_55;
    temp_65 = temp_35 * temp_55;
    temp_66 = temp_37 * temp_55;
    temp_67 = 0. - temp_62;
    temp_68 = fma(temp_44, temp_51, temp_67);
    temp_69 = 0. - temp_63;
    temp_70 = fma(temp_49, temp_51, temp_69);
    temp_71 = temp_36 * temp_59;
    temp_72 = temp_38 * temp_59;
    temp_73 = temp_40 * temp_59;
    temp_74 = temp_64;
    temp_75 = temp_65;
    temp_76 = temp_66;
    if (temp_61) {
        temp_77 = 0. - temp_64;
        temp_78 = temp_77 + -0.;
        temp_74 = temp_78;
    }
    temp_79 = temp_74;
    if (temp_61) {
        temp_80 = 0. - temp_65;
        temp_81 = temp_80 + -0.;
        temp_75 = temp_81;
    }
    temp_82 = temp_75;
    if (temp_61) {
        temp_83 = 0. - temp_66;
        temp_84 = temp_83 + -0.;
        temp_76 = temp_84;
    }
    temp_85 = temp_76;
    temp_86 = temp_30;
    temp_87 = temp_31;
    temp_88 = temp_32;
    temp_89 = 0;
    if (temp_60) {
        temp_90 = 0. - temp_79;
        temp_91 = temp_71 * temp_90;
        temp_92 = 0. - temp_82;
        temp_93 = fma(temp_72, temp_92, temp_91);
        temp_94 = 0. - temp_85;
        temp_95 = fma(temp_73, temp_94, temp_93);
        temp_96 = abs(temp_95);
        temp_97 = 0. - temp_96;
        temp_98 = temp_97 + 1.;
        temp_99 = log2(temp_98);
        temp_100 = U_RimBloomCalc.gRimBloomCalcWork[1].x * 10.;
        temp_101 = temp_100 * temp_99;
        temp_102 = exp2(temp_101);
        temp_103 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_104 = fma(temp_30, temp_103, temp_30);
        temp_105 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_106 = fma(temp_31, temp_105, temp_31);
        temp_107 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_108 = fma(temp_32, temp_107, temp_32);
        temp_109 = 0. - temp_104;
        temp_110 = temp_109 + U_RimBloomCalc.gRimBloomCalcWork[0].x;
        temp_111 = 0. - temp_106;
        temp_112 = temp_111 + U_RimBloomCalc.gRimBloomCalcWork[0].y;
        temp_113 = 0. - temp_108;
        temp_114 = temp_113 + U_RimBloomCalc.gRimBloomCalcWork[0].z;
        temp_115 = temp_102 * U_RimBloomCalc.gRimBloomCalcWork[1].z;
        temp_116 = fma(temp_110, temp_115, temp_104);
        temp_117 = fma(temp_112, temp_115, temp_106);
        temp_118 = fma(temp_114, temp_115, temp_108);
        temp_86 = temp_116;
        temp_87 = temp_117;
        temp_88 = temp_118;
        temp_89 = floatBitsToInt(temp_115);
    }
    temp_119 = temp_86;
    temp_120 = temp_87;
    temp_121 = temp_88;
    temp_122 = temp_89;
    temp_123 = temp_27 + U_VolTexCalc.gVolTexCalcWork[4].y;
    temp_124 = temp_6 < temp_123;
    temp_125 = !temp_124;
    temp_126 = temp_119;
    temp_127 = temp_120;
    temp_128 = temp_121;
    temp_129 = temp_122;
    temp_130 = temp_27;
    temp_131 = temp_125;
    if (temp_124) {
        temp_126 = U_VolTexCalc.gVolTexCalcWork[9].x;
    }
    temp_132 = temp_126;
    temp_133 = temp_132;
    temp_134 = temp_132;
    if (temp_124) {
        temp_127 = U_VolTexCalc.gVolTexCalcWork[9].y;
    }
    temp_135 = temp_127;
    temp_136 = temp_135;
    temp_137 = temp_135;
    if (temp_124) {
        temp_128 = U_VolTexCalc.gVolTexCalcWork[9].z;
    }
    temp_138 = temp_128;
    temp_139 = temp_138;
    temp_140 = temp_138;
    if (temp_124) {
        temp_129 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[9].w);
    }
    temp_141 = temp_129;
    temp_142 = temp_141;
    temp_143 = temp_141;
    if (temp_125) {
        temp_130 = temp_123;
    }
    temp_144 = temp_130;
    temp_145 = temp_144;
    if (temp_125) {
        temp_146 = temp_144 + U_VolTexCalc.gVolTexCalcWork[4].x;
        temp_147 = temp_6 < temp_146;
        if (temp_147) {
            temp_148 = 0. - temp_144;
            temp_149 = temp_146 + temp_148;
            temp_150 = 1. / temp_149;
            temp_151 = 0. - temp_144;
            temp_152 = temp_6 + temp_151;
            temp_153 = 0. - U_VolTexCalc.gVolTexCalcWork[9].x;
            temp_154 = U_VolTexCalc.gVolTexCalcWork[8].x + temp_153;
            temp_155 = 0. - U_VolTexCalc.gVolTexCalcWork[9].y;
            temp_156 = U_VolTexCalc.gVolTexCalcWork[8].y + temp_155;
            temp_157 = 0. - U_VolTexCalc.gVolTexCalcWork[9].z;
            temp_158 = U_VolTexCalc.gVolTexCalcWork[8].z + temp_157;
            temp_159 = temp_152 * temp_150;
            temp_160 = 0. - U_VolTexCalc.gVolTexCalcWork[9].w;
            temp_161 = U_VolTexCalc.gVolTexCalcWork[8].w + temp_160;
            temp_162 = fma(temp_154, temp_159, U_VolTexCalc.gVolTexCalcWork[9].x);
            temp_163 = fma(temp_156, temp_159, U_VolTexCalc.gVolTexCalcWork[9].y);
            temp_164 = fma(temp_158, temp_159, U_VolTexCalc.gVolTexCalcWork[9].z);
            temp_165 = fma(temp_161, temp_159, U_VolTexCalc.gVolTexCalcWork[9].w);
            temp_131 = false;
            temp_133 = temp_162;
            temp_139 = temp_164;
            temp_136 = temp_163;
            temp_142 = floatBitsToInt(temp_165);
        }
        temp_166 = temp_131;
        temp_167 = temp_133;
        temp_168 = temp_139;
        temp_169 = temp_136;
        temp_170 = temp_142;
        temp_171 = temp_166;
        temp_172 = temp_167;
        temp_173 = temp_169;
        temp_174 = temp_168;
        temp_175 = temp_170;
        temp_134 = temp_167;
        temp_140 = temp_168;
        temp_137 = temp_169;
        temp_143 = temp_170;
        if (temp_166) {
            temp_145 = temp_146;
        }
        temp_176 = temp_145;
        temp_177 = temp_176;
        if (temp_166) {
            temp_178 = temp_176 + U_VolTexCalc.gVolTexCalcWork[3].w;
            temp_179 = temp_6 < temp_178;
            if (temp_179) {
                temp_171 = false;
            }
            temp_180 = temp_171;
            temp_181 = temp_180;
            if (temp_179) {
                temp_172 = U_VolTexCalc.gVolTexCalcWork[8].x;
            }
            temp_182 = temp_172;
            temp_183 = temp_182;
            temp_134 = temp_182;
            if (temp_179) {
                temp_173 = U_VolTexCalc.gVolTexCalcWork[8].y;
            }
            temp_184 = temp_173;
            temp_185 = temp_184;
            temp_137 = temp_184;
            if (temp_179) {
                temp_174 = U_VolTexCalc.gVolTexCalcWork[8].z;
            }
            temp_186 = temp_174;
            temp_187 = temp_186;
            temp_140 = temp_186;
            if (temp_179) {
                temp_175 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[8].w);
            }
            temp_188 = temp_175;
            temp_189 = temp_188;
            temp_143 = temp_188;
            if (temp_180) {
                temp_177 = temp_178;
            }
            temp_190 = temp_177;
            temp_191 = temp_190;
            if (temp_180) {
                temp_192 = temp_190 + U_VolTexCalc.gVolTexCalcWork[3].z;
                temp_193 = temp_6 < temp_192;
                if (temp_193) {
                    temp_194 = 0. - temp_190;
                    temp_195 = temp_192 + temp_194;
                    temp_196 = 1. / temp_195;
                    temp_197 = 0. - temp_190;
                    temp_198 = temp_6 + temp_197;
                    temp_199 = 0. - U_VolTexCalc.gVolTexCalcWork[8].x;
                    temp_200 = U_VolTexCalc.gVolTexCalcWork[7].x + temp_199;
                    temp_201 = 0. - U_VolTexCalc.gVolTexCalcWork[8].y;
                    temp_202 = U_VolTexCalc.gVolTexCalcWork[7].y + temp_201;
                    temp_203 = 0. - U_VolTexCalc.gVolTexCalcWork[8].z;
                    temp_204 = U_VolTexCalc.gVolTexCalcWork[7].z + temp_203;
                    temp_205 = 0. - U_VolTexCalc.gVolTexCalcWork[8].w;
                    temp_206 = U_VolTexCalc.gVolTexCalcWork[7].w + temp_205;
                    temp_207 = temp_198 * temp_196;
                    temp_208 = fma(temp_200, temp_207, U_VolTexCalc.gVolTexCalcWork[8].x);
                    temp_209 = fma(temp_202, temp_207, U_VolTexCalc.gVolTexCalcWork[8].y);
                    temp_210 = fma(temp_204, temp_207, U_VolTexCalc.gVolTexCalcWork[8].z);
                    temp_211 = fma(temp_206, temp_207, U_VolTexCalc.gVolTexCalcWork[8].w);
                    temp_181 = false;
                    temp_183 = temp_208;
                    temp_187 = temp_210;
                    temp_185 = temp_209;
                    temp_189 = floatBitsToInt(temp_211);
                }
                temp_212 = temp_181;
                temp_213 = temp_183;
                temp_214 = temp_187;
                temp_215 = temp_185;
                temp_216 = temp_189;
                temp_217 = temp_212;
                temp_218 = temp_213;
                temp_219 = temp_215;
                temp_220 = temp_214;
                temp_221 = temp_216;
                temp_134 = temp_213;
                temp_140 = temp_214;
                temp_137 = temp_215;
                temp_143 = temp_216;
                if (temp_212) {
                    temp_191 = temp_192;
                }
                temp_222 = temp_191;
                temp_223 = temp_222;
                if (temp_212) {
                    temp_224 = temp_222 + U_VolTexCalc.gVolTexCalcWork[3].y;
                    temp_225 = temp_6 < temp_224;
                    if (temp_225) {
                        temp_217 = false;
                    }
                    temp_226 = temp_217;
                    temp_227 = temp_226;
                    if (temp_225) {
                        temp_218 = U_VolTexCalc.gVolTexCalcWork[7].x;
                    }
                    temp_228 = temp_218;
                    temp_229 = temp_228;
                    temp_134 = temp_228;
                    if (temp_225) {
                        temp_219 = U_VolTexCalc.gVolTexCalcWork[7].y;
                    }
                    temp_230 = temp_219;
                    temp_231 = temp_230;
                    temp_137 = temp_230;
                    if (temp_225) {
                        temp_220 = U_VolTexCalc.gVolTexCalcWork[7].z;
                    }
                    temp_232 = temp_220;
                    temp_233 = temp_232;
                    temp_140 = temp_232;
                    if (temp_225) {
                        temp_221 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[7].w);
                    }
                    temp_234 = temp_221;
                    temp_235 = temp_234;
                    temp_143 = temp_234;
                    if (temp_226) {
                        temp_223 = temp_224;
                    }
                    temp_236 = temp_223;
                    temp_237 = temp_236;
                    if (temp_226) {
                        temp_238 = temp_236 + U_VolTexCalc.gVolTexCalcWork[3].x;
                        temp_239 = temp_6 < temp_238;
                        if (temp_239) {
                            temp_240 = 0. - temp_236;
                            temp_241 = temp_238 + temp_240;
                            temp_242 = 1. / temp_241;
                            temp_243 = 0. - temp_236;
                            temp_244 = temp_6 + temp_243;
                            temp_245 = 0. - U_VolTexCalc.gVolTexCalcWork[7].x;
                            temp_246 = U_VolTexCalc.gVolTexCalcWork[6].x + temp_245;
                            temp_247 = 0. - U_VolTexCalc.gVolTexCalcWork[7].y;
                            temp_248 = U_VolTexCalc.gVolTexCalcWork[6].y + temp_247;
                            temp_249 = 0. - U_VolTexCalc.gVolTexCalcWork[7].z;
                            temp_250 = U_VolTexCalc.gVolTexCalcWork[6].z + temp_249;
                            temp_251 = 0. - U_VolTexCalc.gVolTexCalcWork[7].w;
                            temp_252 = U_VolTexCalc.gVolTexCalcWork[6].w + temp_251;
                            temp_253 = temp_244 * temp_242;
                            temp_254 = fma(temp_246, temp_253, U_VolTexCalc.gVolTexCalcWork[7].x);
                            temp_255 = fma(temp_248, temp_253, U_VolTexCalc.gVolTexCalcWork[7].y);
                            temp_256 = fma(temp_250, temp_253, U_VolTexCalc.gVolTexCalcWork[7].z);
                            temp_257 = fma(temp_252, temp_253, U_VolTexCalc.gVolTexCalcWork[7].w);
                            temp_227 = false;
                            temp_229 = temp_254;
                            temp_233 = temp_256;
                            temp_231 = temp_255;
                            temp_235 = floatBitsToInt(temp_257);
                        }
                        temp_258 = temp_227;
                        temp_259 = temp_229;
                        temp_260 = temp_233;
                        temp_261 = temp_231;
                        temp_262 = temp_235;
                        temp_263 = temp_258;
                        temp_264 = temp_259;
                        temp_265 = temp_261;
                        temp_266 = temp_260;
                        temp_267 = temp_262;
                        temp_134 = temp_259;
                        temp_140 = temp_260;
                        temp_137 = temp_261;
                        temp_143 = temp_262;
                        if (temp_258) {
                            temp_237 = temp_238;
                        }
                        temp_268 = temp_237;
                        temp_269 = temp_268;
                        if (temp_258) {
                            temp_270 = temp_268 + U_VolTexCalc.gVolTexCalcWork[2].w;
                            temp_271 = temp_6 < temp_270;
                            if (temp_271) {
                                temp_263 = false;
                            }
                            temp_272 = temp_263;
                            temp_273 = temp_272;
                            if (temp_271) {
                                temp_264 = U_VolTexCalc.gVolTexCalcWork[6].x;
                            }
                            temp_274 = temp_264;
                            temp_275 = temp_274;
                            temp_134 = temp_274;
                            if (temp_271) {
                                temp_265 = U_VolTexCalc.gVolTexCalcWork[6].y;
                            }
                            temp_276 = temp_265;
                            temp_277 = temp_276;
                            temp_137 = temp_276;
                            if (temp_271) {
                                temp_266 = U_VolTexCalc.gVolTexCalcWork[6].z;
                            }
                            temp_278 = temp_266;
                            temp_279 = temp_278;
                            temp_140 = temp_278;
                            if (temp_271) {
                                temp_267 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[6].w);
                            }
                            temp_280 = temp_267;
                            temp_281 = temp_280;
                            temp_143 = temp_280;
                            if (temp_272) {
                                temp_269 = temp_270;
                            }
                            temp_282 = temp_269;
                            temp_283 = temp_282;
                            if (temp_272) {
                                temp_284 = temp_282 + U_VolTexCalc.gVolTexCalcWork[2].z;
                                temp_285 = temp_6 < temp_284;
                                if (temp_285) {
                                    temp_286 = 0. - temp_282;
                                    temp_287 = temp_284 + temp_286;
                                    temp_288 = 1. / temp_287;
                                    temp_289 = 0. - temp_282;
                                    temp_290 = temp_6 + temp_289;
                                    temp_291 = 0. - U_VolTexCalc.gVolTexCalcWork[6].x;
                                    temp_292 = U_VolTexCalc.gVolTexCalcWork[5].x + temp_291;
                                    temp_293 = 0. - U_VolTexCalc.gVolTexCalcWork[6].y;
                                    temp_294 = U_VolTexCalc.gVolTexCalcWork[5].y + temp_293;
                                    temp_295 = 0. - U_VolTexCalc.gVolTexCalcWork[6].z;
                                    temp_296 = U_VolTexCalc.gVolTexCalcWork[5].z + temp_295;
                                    temp_297 = 0. - U_VolTexCalc.gVolTexCalcWork[6].w;
                                    temp_298 = U_VolTexCalc.gVolTexCalcWork[5].w + temp_297;
                                    temp_299 = temp_290 * temp_288;
                                    temp_300 = fma(temp_292, temp_299, U_VolTexCalc.gVolTexCalcWork[6].x);
                                    temp_301 = fma(temp_294, temp_299, U_VolTexCalc.gVolTexCalcWork[6].y);
                                    temp_302 = fma(temp_296, temp_299, U_VolTexCalc.gVolTexCalcWork[6].z);
                                    temp_303 = fma(temp_298, temp_299, U_VolTexCalc.gVolTexCalcWork[6].w);
                                    temp_273 = false;
                                    temp_275 = temp_300;
                                    temp_279 = temp_302;
                                    temp_277 = temp_301;
                                    temp_281 = floatBitsToInt(temp_303);
                                }
                                temp_304 = temp_273;
                                temp_305 = temp_275;
                                temp_306 = temp_279;
                                temp_307 = temp_277;
                                temp_308 = temp_281;
                                temp_309 = temp_304;
                                temp_310 = temp_305;
                                temp_311 = temp_307;
                                temp_312 = temp_306;
                                temp_313 = temp_308;
                                temp_134 = temp_305;
                                temp_140 = temp_306;
                                temp_137 = temp_307;
                                temp_143 = temp_308;
                                if (temp_304) {
                                    temp_283 = temp_284;
                                }
                                temp_314 = temp_283;
                                temp_315 = temp_314;
                                if (temp_304) {
                                    temp_316 = temp_314 + U_VolTexCalc.gVolTexCalcWork[2].y;
                                    temp_317 = temp_6 < temp_316;
                                    if (temp_317) {
                                        temp_309 = false;
                                    }
                                    temp_318 = temp_309;
                                    if (temp_317) {
                                        temp_310 = U_VolTexCalc.gVolTexCalcWork[5].x;
                                    }
                                    temp_319 = temp_310;
                                    temp_134 = temp_319;
                                    if (temp_317) {
                                        temp_311 = U_VolTexCalc.gVolTexCalcWork[5].y;
                                    }
                                    temp_320 = temp_311;
                                    temp_137 = temp_320;
                                    if (temp_317) {
                                        temp_312 = U_VolTexCalc.gVolTexCalcWork[5].z;
                                    }
                                    temp_321 = temp_312;
                                    temp_140 = temp_321;
                                    if (temp_317) {
                                        temp_313 = floatBitsToInt(U_VolTexCalc.gVolTexCalcWork[5].w);
                                    }
                                    temp_322 = temp_313;
                                    temp_143 = temp_322;
                                    if (temp_318) {
                                        temp_315 = temp_316;
                                    }
                                    temp_323 = temp_315;
                                    if (temp_318) {
                                        temp_324 = temp_323 + U_VolTexCalc.gVolTexCalcWork[2].x;
                                        temp_325 = temp_6 < temp_324;
                                        if (temp_325) {
                                            temp_326 = 0. - temp_323;
                                            temp_327 = temp_324 + temp_326;
                                            temp_328 = 1. / temp_327;
                                            temp_329 = 0. - temp_323;
                                            temp_330 = temp_6 + temp_329;
                                            temp_331 = 0. - U_VolTexCalc.gVolTexCalcWork[5].x;
                                            temp_332 = temp_119 + temp_331;
                                            temp_333 = 0. - U_VolTexCalc.gVolTexCalcWork[5].y;
                                            temp_334 = temp_120 + temp_333;
                                            temp_335 = 0. - U_VolTexCalc.gVolTexCalcWork[5].z;
                                            temp_336 = temp_121 + temp_335;
                                            temp_337 = 0. - U_VolTexCalc.gVolTexCalcWork[5].w;
                                            temp_338 = intBitsToFloat(temp_122) + temp_337;
                                            temp_339 = temp_330 * temp_328;
                                            temp_340 = fma(temp_332, temp_339, U_VolTexCalc.gVolTexCalcWork[5].x);
                                            temp_341 = fma(temp_334, temp_339, U_VolTexCalc.gVolTexCalcWork[5].y);
                                            temp_342 = fma(temp_336, temp_339, U_VolTexCalc.gVolTexCalcWork[5].z);
                                            temp_343 = fma(temp_338, temp_339, U_VolTexCalc.gVolTexCalcWork[5].w);
                                            temp_134 = temp_340;
                                            temp_140 = temp_342;
                                            temp_137 = temp_341;
                                            temp_143 = floatBitsToInt(temp_343);
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
    temp_344 = temp_134;
    temp_345 = temp_140;
    temp_346 = temp_137;
    temp_347 = temp_143;
    temp_348 = temp_68 * 0.5;
    temp_349 = in_attr4.z;
    temp_350 = temp_70 * 0.5;
    temp_351 = in_attr3.x;
    temp_352 = abs(temp_348);
    temp_353 = abs(temp_350);
    temp_354 = max(temp_352, temp_353);
    temp_355 = in_attr3.y;
    temp_356 = max(temp_354, 1.);
    temp_357 = 1. / temp_356;
    temp_358 = temp_350 * temp_357;
    temp_359 = temp_348 * temp_357;
    temp_360 = in_attr3.z;
    temp_361 = temp_349 * 8.;
    temp_362 = temp_358 >= 0.;
    temp_363 = floor(temp_361);
    temp_364 = temp_359 >= 0.;
    temp_365 = abs(temp_359);
    temp_366 = inversesqrt(temp_365);
    temp_367 = temp_351 * temp_344;
    temp_368 = abs(temp_358);
    temp_369 = inversesqrt(temp_368);
    temp_370 = max(temp_351, temp_355);
    temp_371 = !temp_362;
    temp_372 = temp_371 ? 0. : 1.;
    temp_373 = 1. / temp_366;
    temp_374 = temp_363 * 0.003921569;
    temp_375 = 1. / temp_369;
    temp_376 = !temp_364;
    temp_377 = temp_376 ? 0. : 1.;
    temp_378 = floor(temp_374);
    temp_379 = temp_372 * 0.6666667;
    temp_380 = fma(temp_377, 0.33333334, temp_379);
    temp_381 = temp_360 * temp_345;
    temp_382 = temp_355 * temp_346;
    temp_383 = max(temp_360, temp_370);
    temp_384 = 0. - temp_363;
    temp_385 = temp_361 + temp_384;
    temp_386 = 0. - temp_378;
    temp_387 = temp_374 + temp_386;
    temp_388 = fma(temp_79, 0.5, 0.5);
    temp_389 = fma(temp_82, 0.5, 0.5);
    temp_390 = fma(temp_85, 1000., 0.5);
    temp_391 = temp_378 * 0.003921569;
    temp_392 = temp_380 + 0.01;
    out_attr0.x = temp_367;
    out_attr0.y = temp_382;
    out_attr0.z = temp_381;
    out_attr0.w = intBitsToFloat(temp_347);
    out_attr1.x = U_Mate.gWrkFl4[1].x;
    out_attr1.y = U_Mate.gWrkFl4[0].w;
    out_attr1.z = U_Mate.gWrkFl4[0].x;
    out_attr1.w = 0.25921568;
    out_attr2.x = temp_388;
    out_attr2.y = temp_389;
    out_attr2.z = temp_383;
    out_attr2.w = temp_390;
    out_attr3.x = temp_373;
    out_attr3.y = temp_375;
    out_attr3.z = 0.;
    out_attr3.w = temp_392;
    out_attr4.x = temp_385;
    out_attr4.y = temp_387;
    out_attr4.z = temp_391;
    out_attr4.w = 0.;
    return;
}
