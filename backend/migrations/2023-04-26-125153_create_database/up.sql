-- Your SQL goes here
CREATE TABLE "turtles" (
	"id"	INTEGER,
	"uuid"	TEXT NOT NULL UNIQUE,
	"x"	INTEGER NOT NULL,
	"y"	INTEGER NOT NULL,
	"z"	INTEGER NOT NULL,
	"rotation" TEXT CHECK(rotation IN ('forward', 'backward', 'left', 'right')) NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE "worlds_data" (
	"id"	INTEGER,
	"turtle_id"	INTEGER NOT NULL,
	"x"	INTEGER NOT NULL,
	"y"	INTEGER NOT NULL,
	"z"	INTEGER NOT NULL,
	"name"	TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);
