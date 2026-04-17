FROM maven:3.9-eclipse-temurin-17 AS build
WORKDIR /build
COPY velocity-plugin/pom.xml .
RUN mvn dependency:go-offline -q
COPY velocity-plugin/src ./src
RUN mvn package -q -DskipTests

FROM itzg/mc-proxy:latest
COPY --from=build /build/target/*.jar /server/plugins/novabox-velocity.jar
