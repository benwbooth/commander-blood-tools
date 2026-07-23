[0000]   BLOCK (exit -> @00B3)
[0004]     AWAIT presentation
[0005]     GUARD active_actor == menu.talk (related 40)
[000A]     GUARD rec_0332 == 65535
[000F]     ENDIF
[0010]     SAY ""IMPROVED MENU""
[001C]     SAY "Today CHEF BRONKO has laid on for you :"
[0036]     SAY "Tasty MURFFALO soup Bronko-style ."
[0048]     SAY "MURFFALO kidneys Bronko-style ."
[0058]     SAY "MURFFALO hamburger with Bar-B-Q recycled-oil dip ."
[006E]     SAY "Smooth MURFFALO-chip ice cream ."
[0080]     SAY "Recycled water"
[008C]     SAY "Chef Bronko says ... Burping's bad manners ! ..."
[00A6]     SAY "stop"  '[skip 1]
[00B0]     END PRESENTATION menu.talk
  END
[00B3]   BLOCK (exit -> @0163)
[00B7]     AWAIT presentation
[00B8]     GUARD active_actor == menu.talk (related 40)
[00BD]     GUARD NOT rec_0332 == 65535
[00C3]     ENDIF
[00C4]     SAY ""MENU""
[00CE]     SAY "Today's fare :"
[00DC]     SAY "PLASMA soup HONK-style ."
[00EC]     SAY "WRIGGLER belly in slobber sauce ."
[0100]     SAY "Jellied URTIKAN with MURFFALO bone marrow ."
[0116]     SAY "GLOK eye pie ."
[0126]     SAY "Recycled water"
[0132]     SAY "The chef says ... Don't eat with your mouth full ! ..."
[0152]     SAY "Stop"  '[skip 2]
[015C]     POKE [0x00B4] = 0
[0160]     END PRESENTATION menu.talk
  END
[0163]   BLOCK (exit -> @020D)
[0167]     AWAIT presentation
[0168]     GUARD active_actor == menu.talk (related 40)
[016D]     GUARD NOT rec_0332 == 65535
[0173]     ENDIF
[0174]     SAY ""MENU""
[017E]     SAY "Today's fare :"
[018C]     SAY "PLASMA soup HONK-style ."
[019C]     SAY "WRIGGLER snout stew ."
[01AC]     SAY "URTIKAN seeds in MURFFALO venom ."
[01C0]     SAY "GLOK juice dessert ."
[01D0]     SAY "Recycled water"
[01DC]     SAY "The chef says ... Don't talk with your mouth open ! ..."
[01FC]     SAY "stop"  '[skip 2]
[0206]     POKE [0x0164] = 0
[020A]     END PRESENTATION menu.talk
  END
[020D]   BLOCK (exit -> @02B9)
[0211]     AWAIT presentation
[0212]     GUARD NOT rec_0332 == 65535
[0218]     GUARD active_actor == menu.talk (related 40)
[021D]     ENDIF
[021E]     SAY ""MENU""
[0228]     SAY "Today's fare :"
[0236]     SAY "PLASMA soup HONK-style ."
[0246]     SAY "WRIGGLER feet in emulsive sauce ."
[025A]     SAY "URTIKAN leaves in MURFFALO sweat ."
[026E]     SAY "GLOK flake dessert ."
[027E]     SAY "Recycled water"
[028A]     SAY "The chef says ... Somebody didn't finish his wrigglers yesterday ..."
[02A8]     SAY "stop"  '[skip 2]
[02B2]     POKE [0x020E] = 0
[02B6]     END PRESENTATION menu.talk
  END
[02B9]   BLOCK (exit -> @036D)
[02BD]     AWAIT presentation
[02BE]     GUARD active_actor == menu.talk (related 40)
[02C3]     GUARD NOT rec_0332 == 65535
[02C9]     ENDIF
[02CA]     SAY ""MENU""
[02D4]     SAY "Today's fare :"
[02E2]     SAY "HONK-style PLASMA soup ."
[02F2]     SAY "WRIGGLER brain , stewed in its own juice ."
[030C]     SAY "URTIKAN trunk , stuffed with MURFFALO liver ."
[0324]     SAY "GLOK dee-lite ."
[0332]     SAY "Recycled water"
[033E]     SAY "The chef says ... Plenty more in the kitchen ! ..."
[035C]     SAY "stop"  '[skip 2]
[0366]     POKE [0x02BA] = 0
[036A]     END PRESENTATION menu.talk
  END
[036D]   BLOCK (exit -> @0429)
[0371]     AWAIT presentation
[0372]     GUARD active_actor == menu.talk (related 40)
[0377]     GUARD NOT rec_0332 == 65535
[037D]     ENDIF
[037E]     SAY ""IMPROVED MENU""
[038A]     SAY "Today's fare :"
[0398]     SAY "Soup of PLASMA HONK-style ."
[03AA]     SAY "WRIGGLER hearts in green blood coagulate ."
[03C0]     SAY "URTIKAN roots , deep fried in recycled oil ."
[03DA]     SAY "Candied GLOK tongue ."
[03EA]     SAY "Recycled water"
[03F6]     SAY "The chef says ... You eat what you are ! ..."
[0414]     SAY "stop"  '[skip 3]
[041E]     POKE [0x036E] = 0
[0422]     POKE [0x042A] = 1
[0426]     END PRESENTATION menu.talk
  END
[0429]   GOTO @0446
[042D]   ENDIF
[042E]   POKE [0x00B4] = 1
[0432]   POKE [0x0164] = 1
[0436]   POKE [0x020E] = 1
[043A]   POKE [0x02BA] = 1
[043E]   POKE [0x036E] = 1
[0442]   POKE [0x042A] = 0
[0446]   BLOCK (exit -> @0453)
[044A]     ENDIF
[044B]     state[22] = 5
[044F]     POKE [0x0447] = 0
  END
[0453]   BLOCK (exit -> @0463)
[0457]     GUARD state[22] == 0
[0459]     ENDIF
[045A]     OP_C3 C3 94 05 28 00
[045F]     POKE [0x0454] = 0
  END
[0463]   BLOCK (exit -> @061D)
[0467]     AWAIT presentation
[0468]     GUARD active_actor == Izwalito.talk (related 40)
[046D]     ENDIF
[046E]     SAY "You found the right button . So far so good ..."  '[skip 1]
[048C]     rec_05A0 = 725
[0491]     SAY "Click quick, Cap'n Bob is waiting ... word_65535 explanations game"
[04AF]     IF-BLOCK (exit -> @04C6)
[04B2]       GUARD concept == "game"
[04B5]       ENDIF
[04B6]       SAY "GAME"  '[skip 3]
[04C0]       RUN PROFILE 1
[04C2]       CLEAR concept_alt
[04C3]       END PRESENTATION Izwalito.talk
    END
[04C6]     IF-BLOCK (exit -> @04CE)
[04C9]       GUARD concept == "explanations"
[04CC]       ENDIF
[04CD]       CLEAR concept_alt
    END
[04CE]     SAY "You can modify the text speed by clicking on "OPTION""
[04EA]     SAY "You can wake Cap'n Bob by clicking on "CRYOBOX""
[0504]     SAY "Or load a "Saved Game" by clicking on "OPTION""
[051E]     SAY "The OPTION "LAST" is the "AUTOMATIC PERMANENT SAVE". If you forget to save, click on "LAST"..."
[0546]     SAY "We would like to draw your ATTENTION to certain RISKS ..."
[0564]     SAY "If you experience UNUSUAL SENSATIONS , DIZZINESS or FACIAL DISTORTION ..."
[0582]     SAY "Stay CALM ..."
[0590]     SAY "It may only be a CRISIS, a SPELL , a CURSE or , at worst , a SCRAMBLED BRAIN ..."
[05C0]     SAY "In which case , write to : SUPER ZEN , end of corridor , CRAZYSTONE planet , BABY1 UNIVERSE ..."
[05F0]     SAY "Click quick on "HONK" . He has important information for YOU ..."
[0610]     SAY "..."  '[skip 1]
[061A]     END PRESENTATION Izwalito.talk
  END
[061D]   BLOCK (exit -> @077D)
[0621]     AWAIT presentation
[0622]     GUARD active_actor == Honk.talk (related 40)
[0627]     ENDIF
[0628]     SAY "Welcome aboard the ARK , Commander . I'm HONK , your trusted computer ..."
[064C]     SAY "I'm here to help you a lot ..."
[0664]     SAY "If the phone rings , just hit the "RED BUTTON" in front of you ..."
[068A]     SAY "Cap'n Bob, our revered leader , is cryonized in the CRYOBOX ..."
[06AA]     SAY "Of course you can wake Cap'n Bob and question him ... But like not too often , okay ? ..."
[06DA]     SAY "Cap'n Bob knows everything ... That's why he's the boss ..."
[06F8]     SAY "Our ship is currently surrounded by things called stars ..."
[0714]     SAY "Remember , deep space is no place for night clubs ! ..."
[0734]     SAY "If you have questions, I have all the answers ..."
[0750]     SAY "Click quick on "CRYOBOX" Cap'n Bob is waiting ..."
[076A]     SAY "End of report ..."  '[skip 1]
[077A]     END PRESENTATION Honk.talk
  END
[077D]   BLOCK (exit -> @0BF6)
[0781]     AWAIT gameflag_274F
[0782]     GUARD active_actor == Bob_Morlock.talk (related 40)
[0787]     ENDIF
[0788]     SAY "Good day COMMANDER . My name is BOB , BOB MORLOCK ..."  '[voice 3]
[07A8]     SAY "If the phone rings , press the "RED BUTTON" on the radio to answer ..."  '[voice 1]
[07CE]     SAY "My ears are fragile, Commander ..."  '[voice 2]
[07E2]     SAY "Do you want me to explain your mission to you , Commander ? word_65535 yes no"  '[voice 2]
[080C]     IF-BLOCK (exit -> @0834)
[080F]       GUARD concept == "no"
[0812]       ENDIF
[0813]       SAY "I'm going to explain it all the same ... It's important ..."  '[voice 6, skip 1]
[0833]       CLEAR concept_alt
    END
[0834]     IF-BLOCK (exit -> @083D)
[0837]       GUARD NOT concept == "no"
[083B]       ENDIF
[083C]       CLEAR concept_alt
    END
[083D]     SAY "I'm more than 800,000 years old ... I feel ancient , Commander ..."  '[voice 1]
[085F]     SAY "Too ancient to go looking for adventure . That's where you come in : I DON'T WANT TO DIE STUPID . I WANT TO SEE THE BIG BANG ..."  '[voice 2]
[08A1]     SAY "This ship , specially built for you , is called the ARK . It's a true marvel of technology ."  '[voice 1]
[08D1]     SAY "Just hit the "SPACE" bar if you've had enough ..."  '[voice 3, skip 3]
[08ED]     LOADSTR "aarche10.hnm"
[08FC]     SAY ""  '[skip 1]
[0904]     LOADSTR "aarche20.hnm"
[0913]     SAY ""  '[skip 1]
[091B]     LOADSTR "aarche30.hnm"
[092A]     SAY ""  '[skip 1]
[0932]     LOADSTR "aarche40.hnm"
[0941]     SAY "I want to understand the world and WHERE I COME FROM ... WHY I EXIST ..."  '[voice 5]
[0969]     SAY "You and me are going to travel through SPACE and TIME, through BLACK HOLES ..."  '[voice 2]
[098F]     SAY "And quite a few BLACK HOLES later , we'll get to the BIG BANG ..."  '[voice 1]
[09B5]     SAY "The tough part will be finding those BLACK HOLES , Commander ..."  '[voice 2]
[09D5]     SAY "That's because BLACK HOLES are UNDETECTABLE in the universe . We'll need to make contact with ALIEN BEINGS ..."  '[voice 1]
[0A03]     SAY "You can expect to see bloodchilling WARS and astonishing PHENOMENA..."  '[voice 4]
[0A1F]     SAY "My age dictates I sleep through most of it in the CRYOBOX . WAKE ME ONLY IN AN EMERGENCY . My time is very precious ..."  '[voice 4]
[0A5B]     SAY "I have provided you with an ONBOARD COMPUTER called HONK ..."  '[voice 2]
[0A79]     SAY "YOU THERE , HONK ?"  '[voice 2]
[0A8B]     SAY "HONK: Yes sir, Cap'n Bob . Present , willing and able ..."
[0AAB]     SAY "Why, you hunk 'o junk ... YOU WERE ASLEEP !!!"  '[voice 5]
[0AC7]     SAY "HONK: No , Cap'n Bob . I was calculating our trajectory to the black hole ! I swear it on my old mother's head !"
[0B01]     SAY "Your mother !! I've a good mind to short-circuit every wire in your lazy carcass ! Keep an eye on him , Commander ..."  '[voice 2]
[0B39]     SAY "GET TO WORK , HONK !!!"  '[voice 5]
[0B4D]     SAY "HONK: Sure thing, Cap'n Bob ... I'm crunching numbers as I speak ..."
[0B6F]     SAY "I can't see a thing ... Feel so weak ..."  '[voice 9]
[0B8B]     SAY "IF YOU NEED ME , WAKE ME UP ..."  '[voice 2]
[0BA5]     SAY "See you later , Commander . Or do I mean earlier ? I'm cryonizing ... Aaaahhhh !"  '[voice 4]
[0BCF]     SAY "The old turkey's out for the count ..."
[0BE7]     SAY "stop"  '[skip 2]
[0BF1]     RUN PROFILE 1
[0BF3]     END PRESENTATION Bob_Morlock.talk
  END
[0BF6]   BLOCK (exit -> @0C0B)
[0BFA]     GUARD rec_0860 == 1
[0C01]     GUARD rec_0080 == 1
[0C08]     ENDIF
[0C09]     RUN PROFILE 1
  END
[0C0B] END OF SCRIPT
