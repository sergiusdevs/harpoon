package dev.harpoon.fixture;

import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.core.annotation.Order;
import org.springframework.security.config.Customizer;
import org.springframework.security.config.annotation.web.builders.HttpSecurity;
import org.springframework.security.config.http.SessionCreationPolicy;
import org.springframework.security.core.userdetails.User;
import org.springframework.security.core.userdetails.UserDetailsService;
import org.springframework.security.provisioning.InMemoryUserDetailsManager;
import org.springframework.security.web.SecurityFilterChain;
import org.springframework.security.web.csrf.CookieCsrfTokenRepository;
import org.springframework.security.web.csrf.CsrfTokenRequestAttributeHandler;

@Configuration
class Security {
    @Bean UserDetailsService users() {
        return new InMemoryUserDetailsManager(User.withUsername("demo").password("{noop}secret").roles("USER").build());
    }
    // API de pruebas sin sesión; los endpoints /session mantienen CSRF habilitado.
    @Bean @Order(1) SecurityFilterChain api(HttpSecurity http) throws Exception {
        return http.securityMatcher("/api/**", "/graphql")
            .csrf(csrf -> csrf.disable())
            .sessionManagement(session -> session.sessionCreationPolicy(SessionCreationPolicy.STATELESS))
            .authorizeHttpRequests(auth -> auth.requestMatchers("/api/private").authenticated().requestMatchers("/api/admin").hasRole("ADMIN").anyRequest().permitAll())
            .httpBasic(Customizer.withDefaults()).build();
    }
    @Bean SecurityFilterChain session(HttpSecurity http) throws Exception {
        return http.csrf(csrf -> csrf.csrfTokenRepository(CookieCsrfTokenRepository.withHttpOnlyFalse()).csrfTokenRequestHandler(new CsrfTokenRequestAttributeHandler()))
            .authorizeHttpRequests(auth -> auth.requestMatchers("/session/csrf", "/session/login", "/error").permitAll().anyRequest().authenticated())
            .formLogin(form -> form.loginProcessingUrl("/session/login").successHandler((request, response, authentication) -> { response.setContentType("application/json"); response.getWriter().write("{\"authenticated\":true}"); }).failureHandler((request, response, exception) -> response.setStatus(401)))
            .exceptionHandling(errors -> errors.authenticationEntryPoint((request, response, exception) -> response.setStatus(401)))
            .build();
    }
}
